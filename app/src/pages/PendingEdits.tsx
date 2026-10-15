import { ChangeTree } from "../components/ChangeTree";
import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { sendBuilds } from "../lib/modManager";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { awaitFieldWrites, queueFieldWrite } from "../lib/fieldWrites";
import { refreshFields } from "../lib/useFieldEdit";
import type { ProjectOverviewDto } from "../bindings/ProjectOverviewDto";
import { useProjectOverview } from "../lib/projectOverview";
import { useSettings } from "../lib/settings";
import { ProjectsStorageField } from "../components/ProjectLocationFields";
import { ProjectFolderField } from "../components/ProjectFolderField";
import { ProjectInputsCard } from "../components/ProjectInputsCard";
import { ProjectRecovery } from "../components/ProjectRecovery";
import { CopiedTargetPanel } from "../components/CopiedTargetPanel";
import { notifyProjectSaved } from "../lib/saveNotice";
import { Button, Card, Pill } from "../components/primitives";
import type { ProjectDetailsDto } from "../bindings/ProjectDetailsDto";

// Keep the existing route valid for saved projects and old links.
export function PendingEdits() {
  const { project, info, update } = useProject();
  const id = project!.project_id;
  const review = useProjectOverview();
  const qc = useQueryClient();
  const revert = useMutation({
    mutationFn: (changeId: string) => queueFieldWrite(id, () => call("project_revert_change", { projectId: id, changeId })),
    onMutate: async (changeId) => {
      await qc.cancelQueries({ queryKey: ["project-overview", id] });
      const snapshots = qc.getQueriesData<ProjectOverviewDto>({ queryKey: ["project-overview", id] });
      qc.setQueriesData<ProjectOverviewDto>({ queryKey: ["project-overview", id] }, value => value ? { ...value, changes: value.changes.filter(c => c.id !== changeId) } : value);
      return snapshots;
    },
    onError: (_error, _id, snapshots) => snapshots?.forEach(([key, data]) => qc.setQueryData(key, data)),
    onSettled: async () => {
      await refreshFields(qc, id, update);
      await qc.invalidateQueries({ queryKey: ["project-overview", id] });
    },
  });
  return <div className="max-w-6xl space-y-5">
    <h1 className="text-xl font-semibold text-slate-100">Project Overview</h1>
    <ProjectRecovery key={id} />
    {!info && <p>Open a Studio project to review its changes.</p>}
    {review.isPending && info && <p className="text-sm text-slate-400">Loading project…</p>}
    {review.isError && <div role="alert" className="rounded-lg border border-rose-800 p-4 text-sm text-rose-300">
      Couldn’t load the project overview. {String(review.error)}
      <Button variant="secondary" className="ml-3" onClick={() => review.refetch()}>Retry</Button>
    </div>}
    {review.data && <>
      <ProjectDetails key={id} projectId={id} initial={review.data.details} />
      <Card title="Project changes" actions={<Pill tone={review.data.dirty ? "warn" : "neutral"}>{review.data.dirty ? "Unsaved changes" : "Saved"}</Pill>}>
        <p className="mb-3 text-xs text-slate-400">All changes since this project was created.</p>
        {review.data.notices.map(n => <p key={n} className="mb-3 text-sm text-amber-300">{n}</p>)}
        {revert.isError && <p role="alert" className="mb-3 text-sm text-rose-300">Couldn’t revert edit. {String(revert.error)}</p>}
        {review.data.changes.length > 0 ? <ChangeTree changes={review.data.changes} onRevert={change => revert.mutate(change.id)} reverting={revert.isPending} />
          : <p className="py-6 text-center text-sm text-slate-400">{review.data.notices.length ? "No other changes to show." : "No changes yet. Your edits will appear here."}</p>}
      </Card>
    </>}
    {project?.platform === "ps4" && info?.mode !== "assets" && <details className="rounded-xl border border-slate-800 p-4">
      <summary className="cursor-pointer text-sm font-medium text-slate-200">PS4 copied-game archive output</summary>
      <p className="my-3 text-xs text-slate-400">Prepare database and dictionary changes inside a copied game archive, with review and restore.</p>
      <CopiedTargetPanel key={id} />
    </details>}
  </div>;
}

function ProjectDetails({ projectId, initial }: { projectId: number; initial: ProjectDetailsDto }) {
  const navigate = useNavigate();
  const [actionError, setActionError] = useState<string | null>(null);
  const { settings } = useSettings();
  const { project, update, updateInfo } = useProject();
  const [draft, setDraft] = useState(initial);
  const [saved, setSaved] = useState(initial);
  const qc = useQueryClient();
  const changed = JSON.stringify(draft) !== JSON.stringify(saved);
  const save = useMutation({
    mutationFn: () => call("project_set_details", { projectId, details: draft }),
    onSuccess: result => { setSaved(result); setDraft(result); void qc.invalidateQueries({ queryKey: ["project-overview", projectId] }); },
  });
  // Both outputs capture a saved project, including the details currently in this form.
  const prepare = async () => {
    await awaitFieldWrites(projectId);
    const details = await call("project_set_details", { projectId, details: draft });
    setSaved(details); setDraft(details);
    const result = await call("project_save", { projectId });
    update(result.summary);
    if (!result.saved && !result.nothing_to_save) throw new Error("The project could not be saved. Resolve the save issue before packaging.");
    updateInfo(await call("project_info", { projectId }), undefined, projectId);
    notifyProjectSaved(projectId);
    await qc.invalidateQueries({ queryKey: ["project-overview", projectId] });
  };
  const build = useMutation({ mutationFn: async () => {
    await prepare();
    return call("project_build", { projectId, projectsRoot: settings.projectsRoot });
  } });
  const archive = useMutation({ mutationFn: async () => {
    await prepare();
    return call("project_archive", { projectId, projectsRoot: settings.projectsRoot });
  } });
  const send = useMutation({ mutationFn: async () => {
    await prepare();
    const result = await call("project_build", { projectId, projectsRoot: settings.projectsRoot });
    const buildPaths = result.packages.map(p => p.path);
    if (!buildPaths.length) throw new Error("No compatible build was created.");
    sendBuilds(buildPaths);
    await qc.invalidateQueries({ queryKey: ["mod-library"] });
    navigate("/manager", { state: { buildPaths, gameId: project?.game_id, platforms: result.packages.flatMap(p => p.targets.map(t => t.platform)) } });
  } });
  const busy = save.isPending || build.isPending || archive.isPending || send.isPending;
  const canPackage = !busy && !!settings.projectsRoot && !!draft.name.trim() && !!draft.version.trim();
  const field = (key: keyof ProjectDetailsDto, label: string, max: number) => <label className="block text-xs text-slate-300">
    {label}<input className="mt-1 block w-full rounded-md border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-slate-100"
      value={draft[key]} maxLength={max} disabled={busy} onChange={e => setDraft({ ...draft, [key]: e.target.value })} />
  </label>;
  return <header className="rounded-xl border border-slate-800 bg-slate-900/60 p-5">
    <div className="grid gap-6 lg:grid-cols-[minmax(0,1fr)_230px]">
      <form onSubmit={e => { e.preventDefault(); save.mutate(); }} className="space-y-4">
        <h2 className="text-sm font-semibold text-slate-100">Project Details</h2>
        <div className="grid gap-4 sm:grid-cols-[minmax(0,2fr)_minmax(0,1fr)]">{field("name", "Name", 120)}{field("version", "Version", 40)}</div>
        {field("author", "Author", 200)}
        <label className="block text-xs text-slate-300">Description
          <textarea rows={3} maxLength={4000} disabled={busy} value={draft.description} onChange={e => setDraft({ ...draft, description: e.target.value })}
            className="mt-1 block w-full resize-y rounded-md border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-slate-100" />
        </label>
        <div className="flex items-center gap-3">
          <Button type="submit" variant="secondary" disabled={!changed || busy || !draft.name.trim() || !draft.version.trim()}>{save.isPending ? "Saving…" : "Save details"}</Button>
          <span role="status" className="text-xs text-slate-400">{changed ? "Unsaved details" : save.isSuccess ? "Details saved" : ""}</span>
        </div>
        {save.isError && <p role="alert" className="text-sm text-rose-300">Couldn’t save details. {String(save.error)}</p>}
      </form>
      <div className="space-y-3 lg:border-l lg:border-slate-800 lg:pl-5">
        <Button disabled={!canPackage} onClick={() => { archive.reset(); build.mutate(); }} className="w-full py-3" aria-describedby="build-help">{build.isPending ? "Building…" : "BUILD"}</Button>
        <p id="build-help" className="text-xs text-slate-400">Saves your edits and creates a .nhlmod for each compatible platform.</p>
        <Button disabled={!canPackage} onClick={() => { build.reset(); archive.mutate(); }} variant="secondary" className="w-full" aria-describedby="archive-help">{archive.isPending ? "Packaging…" : "Package Dev Project Files"}</Button>
        <p id="archive-help" className="text-xs text-slate-400">Saves and packages your complete project for sharing.</p>
        <Button disabled={!canPackage} variant="secondary" className="w-full" onClick={() => send.mutate()}>{send.isPending ? "Preparing mod…" : "Send to Mod Manager"}</Button>
        {send.isError && <p role="alert" className="text-xs text-rose-300">{String(send.error)}</p>}
        {!settings.projectsRoot && <ProjectsStorageField />}
        {(build.isError || archive.isError) && <p role="alert" className="break-words text-xs text-rose-300">{String(build.error ?? archive.error)}</p>}
        {build.isSuccess && <div role="status" className="space-y-2 text-xs">
          <p className="text-emerald-300">Build complete.</p>
          {build.data.packages.map(p => <p key={p.path} className="break-all select-text text-slate-300">{p.path}</p>)}
          {!!build.data.packages.length && <div className="flex flex-col gap-2">
            <Button variant="secondary" onClick={() => { setActionError(null); void revealItemInDir(build.data.packages.map(p => p.path)).catch(e => setActionError(String(e))); }}>Open containing folder</Button>

          </div>}
          {build.data.skipped.map(message => <p key={message} className="text-amber-300">Not built: {message}</p>)}
        </div>}
        {actionError && <p role="alert" className="text-xs text-rose-300">{actionError}</p>}
        {archive.isSuccess && <div role="status" className="space-y-2 text-xs">
          <p className="text-emerald-300">Project archive ready.</p>
          <p className="break-all select-text text-slate-300">{archive.data.path}</p>
        </div>}
      </div>
    </div>
    <ProjectFolderField />
    <ProjectInputsCard />
  </header>;
}

export { ChangeTree } from "../components/ChangeTree";

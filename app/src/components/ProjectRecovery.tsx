import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { Button } from "./primitives";

// Recovery appears only for an interrupted project save, never as routine Overview clutter.
export function ProjectRecovery() {
  const { project, info, update } = useProject();
  const qc = useQueryClient();
  const root = info?.working_dir;
  const status = useQuery({ queryKey: ["project-recovery", root], enabled: !!root,
    queryFn: () => call("copied_target_status", { root: root! }) });
  const recover = useMutation({ mutationFn: async () => {
    const result = await call("copied_target_recover", { root: root! });
    const mine = result.affected.find(a => a.project_id === project!.project_id);
    if (mine?.refresh_error || mine?.stale_baseline) throw new Error(mine.refresh_error || mine.stale_baseline!);
    update(await call("project_summary", { projectId: project!.project_id }));
    return result;
  }, onSettled: () => qc.invalidateQueries() });
  if (!root || !project) return null;
  const s = status.data;
  const interrupted = s && !s.apply_allowed && s.active_txn_id != null && !s.pending_handles.length && !s.reservations.length && !s.busy_projects.length;
  if (!interrupted && !status.isError && !recover.isError && !recover.isSuccess) return null;
  return <section className="rounded-lg border border-amber-700 p-4 text-sm">
    {interrupted && <><p>The previous save was interrupted. Recover it before making more edits.</p>
      <Button disabled={recover.isPending || project.dirty} className="mt-3" onClick={() => recover.mutate()}>{recover.isPending ? "Recovering…" : "Recover interrupted save"}</Button>
      {project.dirty && <p className="mt-2">Close without saving and reopen this project before recovery.</p>}</>}
    {status.isError && <p role="alert">Couldn’t check recovery status. {String(status.error)} <Button onClick={() => void status.refetch()}>Retry</Button></p>}
    {recover.isError && <p role="alert" className="mt-2 text-rose-300">Recovery needs attention: {String(recover.error)}</p>}
    {recover.isSuccess && <p role="status" className="text-emerald-300">Recovery finished. {recover.data.status.guidance}</p>}
  </section>;
}

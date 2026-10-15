// ONB-003 §4 — Build mod package: what the saved project could package (per category, with the
// blockers named), which consoles of the title the edits are measured to support, the package
// identity, then one `.nhlmod` file. Every non-empty category is either included or explicitly
// left out — a build never omits anything silently — and a console is declared only when every
// included category is supported on it.

import { useEffect, useMemo, useState } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import { call, type Platform } from "../lib/client";
import { useProject } from "../lib/project";
import { useSettings } from "../lib/settings";
import { releaseBuildLocation } from "../lib/projectLocation";
import { ProjectsStorageField } from "./ProjectLocationFields";
import { CONSOLE_LABEL } from "../lib/catalog";
import { Button, Card, Pill } from "./primitives";
import { formatBytes } from "./ExtractionPanel";
import type { PackageBuildDto } from "../bindings/PackageBuildDto";

const CATEGORY_LABEL: Record<string, string> = {
  cells: "Database cell edits",
  dictionary: "Dictionary texts",
  textures: "Loose texture replacements (with their source images)",
  members: "Archive-member replacements",
  native: "Native-only outputs",
  structural: "Structural changes (rows added or removed, new dictionary keys)",
};

export function BuildPackagePanel() {
  const { project, info } = useProject();
  const { settings } = useSettings();
  const projectId = project!.project_id;
  const inventory = useQuery({
    queryKey: ["package_inventory", projectId, info?.saves, project?.dirty],
    queryFn: () => call("package_inventory", { projectId }),
    enabled: !!info,
  });
  const [include, setInclude] = useState<Record<string, boolean>>({});
  const [platforms, setPlatforms] = useState<Record<string, boolean>>({});
  const [name, setName] = useState("");
  const [version, setVersion] = useState("");
  const [description, setDescription] = useState("");
  const [built, setBuilt] = useState<PackageBuildDto | null>(null);

  const inv = inventory.data;
  useEffect(() => {
    if (!inv) return;
    setInclude((prev) => {
      const next = { ...prev };
      for (const c of inv.categories) {
        if (next[c.category] === undefined) next[c.category] = c.status === "ready";
      }
      return next;
    });
    setPlatforms((prev) => {
      const next = { ...prev };
      for (const t of inv.targets) {
        if (next[t.platform] === undefined) next[t.platform] = t.platform === info?.platform;
      }
      return next;
    });
    setName((n) => n || inv.default_name);
    setVersion((v) => v || inv.default_version);
  }, [inv, info?.platform]);

  const included = useMemo(() => Object.entries(include).filter(([, on]) => on).map(([k]) => k), [include]);
  const excluded = useMemo(() => (inv ? inv.categories.filter((c) => c.count > 0 && !include[c.category]).map((c) => c.category) : []), [inv, include]);
  const chosenPlatforms = useMemo(() => Object.entries(platforms).filter(([, on]) => on).map(([k]) => k), [platforms]);

  const release = releaseBuildLocation(settings.projectsRoot, name.trim(), chosenPlatforms, info?.game_id ?? "");

  /** A console is offered only when every included, non-empty category is supported on it. */
  const platformBlocked = (platform: string): string | null => {
    const t = inv?.targets.find((x) => x.platform === platform);
    if (!t) return "not a console of this title";
    for (const c of t.components) {
      if (include[c.category] && (inv?.categories.find((x) => x.category === c.category)?.count ?? 0) > 0 && c.status === "unsupported") return `${c.category}: ${c.reason}`;
    }
    return null;
  };

  const build = useMutation({
    mutationFn: () =>
      call("package_build", {
        projectId,
        request: { name: name.trim(), version: version.trim(), description: description.trim(), include: included, exclude: excluded, platforms: chosenPlatforms, out_path: release.path },
      }),
    onSuccess: setBuilt,
  });

  if (!info) return null;
  const blockedIncluded = inv?.categories.filter((c) => include[c.category] && c.status === "blocked") ?? [];
  // Refetches retain old inventory data, including after an error. Only offer a build
  // after a successful refresh, and honor session edits even if that response is older.
  const inventoryReady = inventory.isSuccess && !inventory.isFetching;
  const canBuild = !!inv && !release.error && inventoryReady && !project?.dirty && !inv.dirty && inv.blockers.length === 0 && blockedIncluded.length === 0 && included.length > 0 && chosenPlatforms.length > 0 && chosenPlatforms.every((p) => !platformBlocked(p)) && name.trim() !== "" && version.trim() !== "" && !build.isPending;

  return (
    <Card title="Build mod package" subtitle="One distributable .nhlmod file of this project's saved edits, installable with Studio on another machine without this project. The package carries edit intent and your source images, never the game's files.">
      {!settings.projectsRoot && <ProjectsStorageField />}
      {release.error && <p className="text-xs text-amber-300">{release.error}</p>}
      {inventory.isLoading && <p className="text-xs text-slate-400">Measuring the project's saved edits…</p>}
      {inv && inventory.isFetching && <p className="text-xs text-slate-400">Refreshing saved changes before building…</p>}
      {inv && project?.dirty && !inv.dirty && <p className="text-xs text-amber-300">Save the project's unsaved edits before building a package.</p>}
      {inventory.isError && <p className="text-xs text-rose-300">{String(inventory.error)}</p>}
      {inv && (
        <div className="space-y-3 text-xs" data-testid="build-package">
          {inv.blockers.map((b, i) => (
            <p key={i} className="text-amber-300">{b}</p>
          ))}
          <div>
            <div className="mb-1 font-medium text-slate-200">Contents</div>
            <ul className="space-y-1">
              {inv.categories
                .filter((c) => c.count > 0)
                .map((c) => (
                  <li key={c.category} className="rounded-md border border-slate-800 bg-slate-950/40 p-2">
                    <label className="flex cursor-pointer items-center gap-2 text-slate-200">
                      <input type="checkbox" checked={!!include[c.category]} disabled={c.status === "blocked"} onChange={(e) => setInclude((s) => ({ ...s, [c.category]: e.target.checked }))} className="h-3 w-3" />
                      {CATEGORY_LABEL[c.category] ?? c.category}
                      <Pill tone={c.status === "ready" ? "ok" : c.status === "blocked" ? "warn" : "neutral"}>
                        {c.count} · {c.status}
                      </Pill>
                      {c.status === "blocked" && <span className="text-slate-500">cannot be packaged — left out explicitly</span>}
                    </label>
                    {c.items.length > 0 && (
                      <details className="mt-1">
                        <summary className="cursor-pointer text-slate-400">show items</summary>
                        <ul className="mt-1 max-h-40 list-disc overflow-y-auto pl-5 text-slate-400">
                          {c.items.map((it, i) => (
                            <li key={i}>{it}</li>
                          ))}
                        </ul>
                      </details>
                    )}
                    {c.blockers.length > 0 && (
                      <ul className="mt-1 list-disc pl-5 text-amber-300">
                        {c.blockers.slice(0, 6).map((b, i) => (
                          <li key={i}>{b}</li>
                        ))}
                        {c.blockers.length > 6 && <li>… {c.blockers.length - 6} more</li>}
                      </ul>
                    )}
                  </li>
                ))}
              {inv.categories.every((c) => c.count === 0) && <li className="text-slate-500">The project has no saved edits yet.</li>}
            </ul>
          </div>
          <div>
            <div className="mb-1 font-medium text-slate-200">Consoles this package declares</div>
            <ul className="space-y-1">
              {inv.targets.map((t) => {
                const why = platformBlocked(t.platform);
                return (
                  <li key={t.platform}>
                    <label className={"flex cursor-pointer items-start gap-2 " + (why ? "text-slate-500" : "text-slate-200")}>
                      <input type="checkbox" checked={!!platforms[t.platform] && !why} disabled={!!why} onChange={(e) => setPlatforms((s) => ({ ...s, [t.platform]: e.target.checked }))} className="mt-0.5 h-3 w-3" />
                      <span>
                        {CONSOLE_LABEL[t.platform as Platform] ?? t.platform}
                        {t.platform === info.platform && <span className="text-slate-500"> (this project)</span>}
                        <div className="text-slate-500">
                          {why ?? t.components.filter((c) => c.status !== "empty").map((c) => `${c.category}: ${c.reason}`).join(" · ")}
                        </div>
                      </span>
                    </label>
                  </li>
                );
              })}
            </ul>
          </div>
          <div className="grid gap-2 sm:grid-cols-3">
            <label className="text-slate-300">
              Name
              <input aria-label="Package name" className="mt-1 w-full rounded-md border border-slate-700 bg-slate-950 px-2 py-1 text-slate-100" value={name} onChange={(e) => setName(e.target.value)} />
            </label>
            <label className="text-slate-300">
              Version
              <input aria-label="Package version" className="mt-1 w-full rounded-md border border-slate-700 bg-slate-950 px-2 py-1 text-slate-100" value={version} onChange={(e) => setVersion(e.target.value)} />
            </label>
            <label className="text-slate-300 sm:col-span-3">
              Description
              <input aria-label="Package description" className="mt-1 w-full rounded-md border border-slate-700 bg-slate-950 px-2 py-1 text-slate-100" value={description} onChange={(e) => setDescription(e.target.value)} />
            </label>
          </div>
          <div className="flex items-center gap-2">
            <Button onClick={() => build.mutate()} disabled={!canBuild}>
              {build.isPending ? "Building…" : "Build package"}
            </Button>
            <span className="text-slate-500">Output: {release.path || "Choose a storage folder and build name"}; an existing build is never overwritten.</span>
          </div>
          {build.isError && <p className="text-rose-300">{String(build.error)}</p>}
          {built && (
            <div className="rounded-md border border-emerald-600/40 bg-emerald-500/10 p-2 text-emerald-200" data-testid="build-result">
              Built <span className="font-mono">{built.path}</span> — {formatBytes(built.len)}, {built.entries} file(s), SHA-256 <span className="font-mono">{built.sha256.slice(0, 16)}…</span>; declares {built.targets.map((t) => CONSOLE_LABEL[t.platform as Platform] ?? t.platform).join(", ")}.
            </div>
          )}
        </div>
      )}
    </Card>
  );
}

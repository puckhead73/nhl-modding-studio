import { useEffect, useMemo, useRef, useState } from "react";
import { GamePlatformPill } from "../components/GamePlatformPill";
import { useLocation, useNavigate } from "react-router-dom";
import { useMutation, useQuery } from "@tanstack/react-query";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { call, type Platform } from "../lib/client";
import { useProject } from "../lib/project";
import { CONSOLE_LABEL, GAMES, gameById, needsSavePath, showsSavePath } from "../lib/catalog";
import type { PackageInspectDto } from "../bindings/PackageInspectDto";
import type { InstallPlanDto } from "../bindings/InstallPlanDto";
import type { InstallApplyDto } from "../bindings/InstallApplyDto";
import type { InstallReportDto } from "../bindings/InstallReportDto";
import { Button, Card, ErrorBanner, Pill } from "../components/primitives";
import { formatBytes } from "../components/ExtractionPanel";
import headerLogo from "../assets/ui/NHL Modding Studio Header.png";

/**
 * ONB-003 §5 — Install mod package. Works without a project: pick a package → it is read and
 * validated → choose the destination game / console and its paths → a read-only preflight stages
 * every conversion and shows the exact planned destination changes, requirements and conflicts →
 * Apply writes with backups under an installation record → Restore undoes a recorded installation
 * when the destination still holds exactly what was installed. Nothing is written before Apply.
 */
const inputCls =
  "w-full rounded-md border border-slate-700 bg-slate-950 px-2 py-1.5 text-sm text-slate-100 placeholder-slate-600 focus:border-sky-600 focus:outline-none";

const STATE_LABEL: Record<string, string> = {
  replace: "replaced (backed up first)",
  create: "created",
  unchanged: "already identical — not written",
};

/** The catalog game for a package game id on a platform (Legacy Recompiled is its own entry). */
function gameFor(gameId: string, platform: Platform) {
  if (platform === "recomp") return gameById("legacyrecomp") ?? gameById(gameId);
  if (gameId === "legacyrecomp") return gameById("nhllegacy");
  return gameById(gameId);
}

export function InstallPackagePage() {
  const navigate = useNavigate();
  const location = useLocation();
  const initial = useRef(location.state as { packagePath?: string; gameId?: string; platform?: Platform; root?: string } | null).current;
  const initialized = useRef(false);
  const { project } = useProject();
  const [path, setPath] = useState(initial?.packagePath ?? "");
  const [inspected, setInspected] = useState<PackageInspectDto | null>(null);
  const [platform, setPlatform] = useState<Platform | null>(null);
  const [root, setRoot] = useState(initial?.root ?? "");
  const [savePath, setSavePath] = useState("");
  const [rosterSave, setRosterSave] = useState("");
  const [report, setReport] = useState<InstallReportDto | null>(null);
  const [allowConflicts, setAllowConflicts] = useState(false);
  const [plan, setPlan] = useState<InstallPlanDto | null>(null);
  const [applied, setApplied] = useState<InstallApplyDto | null>(null);

  const inspect = useMutation({
    mutationFn: (p: string) => call("package_inspect", { path: p }),
    onSuccess: (r) => {
      setInspected(r);
      setPlan(null);
      setApplied(null);
      const supported = r.targets.filter((t) => t.status === "supported");
      setPlatform(initial?.platform && supported.some(t => t.platform === initial.platform)
        ? initial.platform : supported.length === 1 ? (supported[0].platform as Platform) : null);
    },
    onError: () => setInspected(null),
  });

  useEffect(() => {
    if (initial?.packagePath && !initialized.current) {
      initialized.current = true;
      inspect.mutate(initial.packagePath);
    }
  }, [initial]);

  const game = inspected && platform ? gameFor(inspected.game_id, platform) : null;
  const gameId = game?.id ?? inspected?.game_id ?? "";

  const detect = useMutation({
    mutationFn: () => call("detect_install", { gameId, platform: platform!, root, savePath: showsSavePath(platform) ? savePath.trim() || null : null }),
    onSuccess: setReport,
  });
  // R72-R1: a roster-only package (every edit targets the roster save; no database, dictionary or
  // texture edit) needs no game installation — the save-data location alone is the destination
  // (PS3 excepted: its saves live inside the RPCS3 folder, which stays the folder to choose).
  const rosterOnlyPkg =
    !!inspected &&
    inspected.cells.length > 0 &&
    inspected.cells.every((c) => c.file === "roster") &&
    inspected.dictionary.length === 0 &&
    inspected.textures.length === 0;
  const rootOptional = rosterOnlyPkg && platform !== "ps3";
  const locationGiven = root.trim() !== "" || (rootOptional && savePath.trim() !== "");
  const needsRoster = (inspected?.cells ?? []).some((c) => c.file !== "main_db");
  const readyToDetect = !!inspected && !!platform && locationGiven && (!needsRoster || !needsSavePath(platform) || savePath.trim() !== "");
  useEffect(() => {
    if (!readyToDetect) return;
    const t = setTimeout(() => detect.mutate(), 400);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [readyToDetect, root, savePath, platform, gameId, needsRoster]);

  // R71-R2: the roster save a package may need is an explicit choice on every platform when
  // several were found; the choice is sent only when it is one of the saves found now, and a
  // package without roster edits never needs one (the plan says "not needed").
  const rosterChoices = useMemo(() => (report ? report.saves.filter((s) => s.kind === "roster" && s.writable) : []), [report]);
  const rosterArg = rosterChoices.find((s) => s.path === rosterSave)?.path ?? (rosterChoices.length === 1 ? rosterChoices[0].path : null);
  const needsRosterChoice = needsRoster && rosterChoices.length > 1 && !rosterArg;

  const preflight = useMutation({
    mutationFn: () =>
      call("package_preflight", {
        packagePath: path.trim(),
        gameId,
        platform: platform!,
        root: root.trim(),
        savePath: showsSavePath(platform) ? savePath.trim() || null : null,
        rosterSave: rosterArg,
        allowConflicts,
      }),
    onSuccess: (p) => {
      setPlan(p);
      setApplied(null);
    },
  });
  const apply = useMutation({
    mutationFn: () => call("package_apply", { handle: plan!.handle }),
    onSuccess: (r) => {
      setApplied(r);
      setPlan(null);
      installs.refetch();
    },
  });
  const installs = useQuery({ queryKey: ["package_installs"], queryFn: () => call("package_installs", {}) });
  const restore = useMutation({
    mutationFn: (recordDir: string) => call("package_restore", { recordDir }),
    onSuccess: () => installs.refetch(),
  });

  async function browsePackage() {
    const picked = await openDialog({ multiple: false, directory: false, title: "Pick the mod package", filters: [{ name: "Studio mod package", extensions: ["nhlmod"] }] });
    if (typeof picked === "string") {
      setPath(picked);
      inspect.mutate(picked);
    }
  }
  async function browseDir(title: string, set: (p: string) => void) {
    const picked = await openDialog({ multiple: false, directory: true, title });
    if (typeof picked === "string") set(picked);
  }

  const canPreflight = !!inspected && !!platform && locationGiven && !needsRosterChoice && !preflight.isPending;
  const changing = plan?.items.filter((i) => i.state !== "unchanged") ?? [];

  return (
    <div className="min-h-full overflow-y-auto bg-slate-950 p-6 text-slate-100">
      <div className="mx-auto max-w-4xl space-y-4">
        <div className="flex items-center justify-between">
          <img src={headerLogo} alt="NHL Modding Studio" className="h-10 w-auto object-contain" />
          <Button variant="secondary" onClick={() => navigate(project ? "/app/settings/general" : "/manager")}>
            {project ? "Back to the project" : "Back to Mod Manager"}
          </Button>
        </div>
        <header>
          <h1 className="text-xl font-semibold">Install mod package</h1>
          <p className="mt-1 text-xs text-slate-400">
            A package is read and checked first, then applied to a destination of your choice only after you review the exact changes and press Install. Every replaced file is backed up and can be restored.
          </p>
        </header>

        <Card title="1. Package">
          <div className="flex gap-2">
            <input aria-label="Package file" className={inputCls + " flex-1 font-mono text-xs"} value={path} onChange={(e) => setPath(e.target.value)} placeholder="…/my-mod-1.0.nhlmod" />
            <Button variant="secondary" onClick={() => void browsePackage()}>
              Browse…
            </Button>
            <Button onClick={() => inspect.mutate(path.trim())} disabled={path.trim() === "" || inspect.isPending}>
              {inspect.isPending ? "Reading…" : "Read package"}
            </Button>
          </div>
          {inspect.isError && (
            <div className="mt-3">
              <ErrorBanner title="The package was refused" message={String(inspect.error)} />
            </div>
          )}
          {inspected && (
            <div className="mt-3 space-y-2 text-xs" data-testid="package-summary">
              <div className="text-sm text-slate-100">
                <b>{inspected.name}</b> {inspected.version}
                {inspected.description && <span className="text-slate-400"> — {inspected.description}</span>}
              </div>
              <div className="text-slate-400">
                For <span className="text-slate-200">{inspected.title}</span>, authored on {CONSOLE_LABEL[inspected.author_platform as Platform] ?? inspected.author_platform} · built with Studio {inspected.studio_version} on {inspected.created} · {formatBytes(inspected.len)} · id <span className="font-mono">{inspected.package_id.slice(0, 12)}…</span>
              </div>
              <div className="flex flex-wrap gap-1.5">
                {inspected.categories.map((c) => (
                  <Pill key={c.category} tone={c.count > 0 ? "info" : "neutral"}>
                    {c.count} {c.category}
                  </Pill>
                ))}
              </div>
              <div>
                <div className="mb-1 font-medium text-slate-200">Declared consoles</div>
                <ul className="space-y-1">
                  {inspected.targets.map((t) => (
                    <li key={t.platform} className="flex flex-wrap items-center gap-2">
                      <Pill tone={t.status === "supported" ? "ok" : "warn"}>{CONSOLE_LABEL[t.platform as Platform] ?? t.platform}</Pill>
                      <span className="text-slate-400">
                        {t.components
                          .filter((c) => c.status !== "empty")
                          .map((c) => `${c.category}: ${c.status}`)
                          .join(" · ") || "no edits"}
                      </span>
                    </li>
                  ))}
                </ul>
              </div>
              {inspected.cells.length > 0 && (
                <details>
                  <summary className="cursor-pointer text-slate-300">{inspected.cells.length} database edit(s)</summary>
                  <ul className="mt-1 max-h-48 list-disc overflow-y-auto pl-4 text-slate-400">
                    {inspected.cells.map((c, i) => (
                      <li key={i}>
                        {c.table_name} › {c.entity} › {c.field_name}: {c.before} → {c.after}
                        {c.file === "roster" ? " (roster save)" : ""}
                      </li>
                    ))}
                  </ul>
                </details>
              )}
              {inspected.dictionary.length > 0 && (
                <details>
                  <summary className="cursor-pointer text-slate-300">{inspected.dictionary.length} dictionary text(s)</summary>
                  <ul className="mt-1 max-h-48 list-disc overflow-y-auto pl-4 text-slate-400">
                    {inspected.dictionary.map((d) => (
                      <li key={d.key}>
                        {d.key}: “{d.before}” → “{d.after}”
                      </li>
                    ))}
                  </ul>
                </details>
              )}
              {inspected.textures.length > 0 && (
                <details>
                  <summary className="cursor-pointer text-slate-300">{inspected.textures.length} texture replacement(s)</summary>
                  <ul className="mt-1 max-h-48 list-disc overflow-y-auto pl-4 text-slate-400">
                    {inspected.textures.map((t) => (
                      <li key={t.rel_path} className="font-mono">
                        {t.rel_path} ({t.width}×{t.height}{t.texture_index != null ? `, texture ${t.texture_index}` : ""})
                      </li>
                    ))}
                  </ul>
                </details>
              )}
              <details>
                <summary className="cursor-pointer text-slate-300">{inspected.entries.length} package file(s)</summary>
                <ul className="mt-1 max-h-48 overflow-y-auto pl-4 font-mono text-slate-500">
                  {inspected.entries.map((e) => (
                    <li key={e.path}>
                      {e.path} · {formatBytes(e.len)} · {e.sha256.slice(0, 12)}…
                    </li>
                  ))}
                </ul>
              </details>
            </div>
          )}
        </Card>

        {inspected && (
          <Card title="2. Destination" subtitle="Where the mod goes: your own game installation (and save location). These are final destinations; nothing is written until Install.">
            <div className="space-y-3 text-xs">
              <div>
                <div className="mb-1 font-medium text-slate-300">Console</div>
                <div className="flex flex-wrap gap-2">
                  {inspected.targets.map((t) => (
                    <label key={t.platform} className={"flex cursor-pointer items-center gap-1.5 rounded-md border px-2 py-1 " + (t.status === "supported" ? "border-slate-700" : "border-slate-800 text-slate-500")}>
                      <input type="radio" name="platform" disabled={t.status !== "supported"} checked={platform === t.platform} onChange={() => { setPlatform(t.platform as Platform); setReport(null); setPlan(null); setRosterSave(""); }} className="h-3 w-3" />
                      {CONSOLE_LABEL[t.platform as Platform] ?? t.platform}
                      {t.status !== "supported" && <span>(not supported by this package)</span>}
                    </label>
                  ))}
                </div>
                {game && <p className="mt-1 text-slate-500">Game: {game.label}</p>}
              </div>
              {platform && (
                <>
                  <PathField label="Game installation folder" value={root} onChange={(v) => { setRoot(v); setReport(null); setPlan(null); }} onBrowse={() => void browseDir("Pick the game installation folder", (v) => { setRoot(v); setReport(null); setPlan(null); })} hint={rootOptional ? "Optional for this package: it edits the roster save only, so the save-data location below is the destination and the game's schema comes from Studio's bundled baselines." : "The destination game folder (RPCS3 folder for PS3, the install folder for Xbox 360, the PS4 game folder, the recompiled game folder)."} />
                  {rootOptional && <p className="text-slate-400" data-testid="roster-only-note">This package edits the roster save only — no game installation is needed.</p>}
                  {showsSavePath(platform) && (
                    <PathField label="Save-data location" value={savePath} onChange={(v) => { setSavePath(v); setReport(null); setPlan(null); }} onBrowse={() => void browseDir("Pick the save-data folder", (v) => { setSavePath(v); setReport(null); setPlan(null); })} hint={platform === "ps4" ? "shadPS4's savedata folder (needed only when the package edits the roster save)." : "The folder holding your roster save (needed only when the package edits the roster save)."} />
                  )}
                  {report && (
                    <div className="rounded-md border border-slate-800 bg-slate-950/40 p-2 text-slate-400" data-testid="destination-report">
                      {!rosterOnlyPkg && <div>{report.nhlng_path ? `Database: ${report.nhlng_path}` : "Database: not found under this folder"}</div>}
                      {!rosterOnlyPkg && <div>{report.localization_path ? `Dictionary: ${report.localization_path}` : "Dictionary: not found"}</div>}
                      {rosterOnlyPkg && <div>Database and dictionary: not needed by this package</div>}
                      <div>Saves found: {report.saves.length}{needsRoster ? "" : " (this package edits no roster save; none is touched)"}</div>
                      {needsRoster && rosterChoices.length > 1 && (
                        <fieldset className="mt-1 space-y-1" data-testid="roster-choice">
                          <legend className="text-slate-300">Several roster saves were found — choose which one receives roster edits:</legend>
                          {rosterChoices.map((s) => (
                            <label key={s.path} className="flex cursor-pointer items-center gap-2">
                              <input type="radio" name="roster" checked={rosterSave === s.path} onChange={() => setRosterSave(s.path)} className="h-3 w-3" />
                              <span className="font-mono">{s.path}</span>
                            </label>
                          ))}
                        </fieldset>
                      )}
                    </div>
                  )}
                  {detect.isError && <p className="text-rose-300">{String(detect.error)}</p>}
                  <div className="flex items-center gap-3">
                    <Button onClick={() => preflight.mutate()} disabled={!canPreflight}>
                      {preflight.isPending ? "Checking…" : "Review installation"}
                    </Button>
                    <label className="flex cursor-pointer items-center gap-2 text-slate-300">
                      <input type="checkbox" checked={allowConflicts} onChange={(e) => { setAllowConflicts(e.target.checked); setPlan(null); }} className="h-3 w-3" />
                      Allow replacing values this destination changed on its own
                    </label>
                  </div>
                  {preflight.isError && <ErrorBanner title="The installation cannot be planned" message={String(preflight.error)} />}
                </>
              )}
            </div>
          </Card>
        )}

        {plan && (
          <Card title="3. Planned changes" subtitle="Exactly what Install would write. Nothing has been written yet.">
            <div className="space-y-2 text-xs" data-testid="install-plan">
              <div className="flex flex-wrap gap-1.5">
                <Pill tone="info">{plan.applied} edit(s) to apply</Pill>
                {plan.already_present > 0 && <Pill tone="neutral">{plan.already_present} already present</Pill>}
                {plan.conflicts.length > 0 && <Pill tone="warn">{plan.conflicts.length} conflict(s)</Pill>}
              </div>
              <ul className="space-y-1">
                {plan.requirements.map((r) => (
                  <li key={r.role} className="flex items-start gap-2">
                    <Pill tone={r.status === "found" ? "ok" : r.status === "missing" ? "error" : "neutral"}>{r.role.replace("_", " ")}{r.status === "not-needed" ? " · not needed" : r.status === "missing" ? " · missing" : ""}</Pill>
                    <span className="font-mono text-slate-400">{r.detail}</span>
                  </li>
                ))}
              </ul>
              {plan.conflicts.length > 0 && (
                <ul className="list-disc pl-4 text-amber-300">
                  {plan.conflicts.map((c, i) => (
                    <li key={i}>{c}</li>
                  ))}
                </ul>
              )}
              {plan.reasons.length > 0 && (
                <ul className="list-disc pl-4 text-rose-300">
                  {plan.reasons.map((r, i) => (
                    <li key={i}>{r}</li>
                  ))}
                </ul>
              )}
              <table className="w-full">
                <thead className="text-slate-400">
                  <tr>
                    <th className="py-1 pr-2 text-left">File</th>
                    <th className="py-1 pr-2 text-left">Destination</th>
                    <th className="py-1 pr-2 text-left">Outcome</th>
                  </tr>
                </thead>
                <tbody>
                  {plan.items.map((i) => (
                    <tr key={i.key} className="border-t border-slate-800 align-top">
                      <td className="py-1 pr-2 text-slate-200">
                        {i.label}
                        <div className="font-mono text-[11px] text-slate-500">{i.replacement.sha256.slice(0, 12)}… · {formatBytes(i.replacement.len)}</div>
                      </td>
                      <td className="py-1 pr-2 font-mono text-[11px] text-slate-400">{i.destination_path}</td>
                      <td className="py-1 pr-2">
                        <Pill tone={i.state === "unchanged" ? "neutral" : "info"}>{STATE_LABEL[i.state] ?? i.state}</Pill>
                        {i.notes.map((n, k) => (
                          <div key={k} className="mt-1 text-amber-300">{n}</div>
                        ))}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
              <div className="text-slate-400">
                {changing.length} file(s) would be written ({formatBytes(plan.replacement_bytes)}); {formatBytes(plan.backup_bytes)} of backups kept under {plan.records_dir}.
              </div>
              <div className="flex items-center gap-2">
                <Button onClick={() => apply.mutate()} disabled={!plan.installable || apply.isPending}>
                  {apply.isPending ? "Installing…" : "Install"}
                </Button>
                <Button variant="secondary" onClick={() => setPlan(null)} disabled={apply.isPending}>
                  Cancel
                </Button>
                {apply.isError && <span className="text-rose-300">{String(apply.error)}</span>}
              </div>
            </div>
          </Card>
        )}

        {applied && (
          <div className="rounded-md border border-emerald-600/40 bg-emerald-500/10 p-3 text-xs text-emerald-200" data-testid="install-result">
            Installation {applied.seq} {applied.state}: {applied.destinations.length} destination(s). Backups and the record are in <span className="font-mono">{applied.record_dir}</span>.
          </div>
        )}

        <Card title="Installed packages" subtitle="Recorded installations on this machine. Restore puts the backed-up originals back when the destination still holds exactly what was installed; an interrupted installation or restore is finished by Recover once its destination is reachable again.">
          {installs.data && installs.data.length === 0 && <p className="text-xs text-slate-500">Nothing has been installed yet.</p>}
          {installs.data && installs.data.length > 0 && (
            <ul className="space-y-1 text-xs">
              {installs.data.map((r) => (
                <li key={r.record_dir} className="flex flex-wrap items-center gap-2 text-slate-300">
                  <span>#{r.seq}</span>
                  <span className="text-slate-500">{r.at}</span>
                  <span>
                    {r.package_name} {r.package_version}
                  </span>
                  <GamePlatformPill gameId={r.game_id} platform={r.platform} />
                  <Pill tone={r.state === "committed" ? "ok" : r.state === "restored" ? "neutral" : "warn"}>{r.state}</Pill>
                  {r.restorable && (
                    <Button variant="ghost" onClick={() => restore.mutate(r.record_dir)} disabled={restore.isPending}>
                      {r.recovery ? "Recover" : "Restore originals"}
                    </Button>
                  )}
                </li>
              ))}
            </ul>
          )}
          {restore.isError && <p className="mt-1 text-xs text-rose-300">{String(restore.error)}</p>}
          {restore.isSuccess && <p className="mt-1 text-xs text-emerald-300">Restored: {restore.data.destinations.length} destination(s) hold their originals again.</p>}
        </Card>
        <p className="text-[11px] text-slate-500">Available titles: {GAMES.map((g) => g.label).join(", ")}. A package names its own title and consoles; the installer never converts between titles.</p>
      </div>
    </div>
  );
}

function PathField({ label, value, onChange, onBrowse, hint }: { label: string; value: string; onChange: (v: string) => void; onBrowse: () => void; hint?: string }) {
  return (
    <div>
      <label className="mb-1 block font-medium text-slate-300">{label}</label>
      <div className="flex gap-2">
        <input aria-label={label} className={inputCls + " flex-1 font-mono text-xs"} value={value} onChange={(e) => onChange(e.target.value)} />
        <Button variant="secondary" onClick={onBrowse}>
          Browse…
        </Button>
      </div>
      {hint && <p className="mt-1 text-[11px] text-slate-500">{hint}</p>}
    </div>
  );
}

import { ChangeTree } from "../components/ChangeTree";
import { useEffect, useRef, useState } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { call, type Platform } from "../lib/client";
import { CONSOLE_LABEL, GAMES, gameById, img, PLATFORM_ORDER } from "../lib/catalog";
import { sameManagerPath, containingFolder, loadSentBuilds, targetInstallationRecords, installationRecords, loadManager, MANAGER_KEY, matchesTarget, targetKey, type ManagerPreferences, type ManagerTarget } from "../lib/modManager";
import { useProject } from "../lib/project";
import { SessionControls } from "../components/SessionControls";
import { Button, Pill } from "../components/primitives";
import { GamePlatformPill } from "../components/GamePlatformPill";
import headerLogo from "../assets/ui/NHL Modding Studio Header.png";

const fieldClass = "w-full rounded-md border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-slate-100";
type SetupMode = "first" | "library" | "target";

export function ModManagerPage() {
  const navigate = useNavigate();
  const { project } = useProject();
  const incoming = useLocation().state as { buildPaths?: string[]; gameId?: string; platforms?: string[] } | null;
  const [sentBuilds] = useState(loadSentBuilds);
  const qc = useQueryClient();
  const [prefs, setPrefs] = useState(() => {
    const saved = loadManager();
    const match = saved.targets.find(t => t.gameId === incoming?.gameId && incoming.platforms?.includes(t.platform));
    return match ? { ...saved, selected: targetKey(match) } : saved;
  });
  const [setup, setSetup] = useState<SetupMode | null>(() => !prefs.downloads || !prefs.targets.length ? "first" : incoming?.gameId && !prefs.targets.some(t => t.gameId === incoming.gameId && incoming.platforms?.includes(t.platform)) ? "target" : null);
  const [editTarget, setEditTarget] = useState<ManagerTarget | undefined>();
  const [selected, setSelected] = useState<string | null>(incoming?.buildPaths?.[0] ?? null);
  const [storageError, setStorageError] = useState<string | null>(null);
  const target = prefs.targets.find(t => targetKey(t) === prefs.selected) ?? prefs.targets[0];
  const savePreferences = (next: ManagerPreferences) => {
    try { localStorage.setItem(MANAGER_KEY, JSON.stringify(next)); setStorageError(null); }
    catch { setStorageError("Your settings could not be remembered on this device. They will work for this session."); }
    setPrefs(next);
  };
  const library = useQuery({ queryKey: ["mod-library", prefs.downloads, sentBuilds],
    queryFn: async () => {
      const main = await call("mod_library_scan", { root: prefs.downloads });
      const entries = new Map(main.mods.map(m => [m.path, m]));
      const warnings = [...main.warnings];
      for (const root of new Set(sentBuilds.map(containingFolder))) {
        try {
          const extra = await call("mod_library_scan", { root });
          for (const mod of extra.mods) if (sentBuilds.some(path => sameManagerPath(path, mod.path)) && ![...entries.keys()].some(path => sameManagerPath(path, mod.path))) entries.set(mod.path, mod);
        } catch (error) { warnings.push(`Couldn’t read a sent build: ${String(error)}`); }
      }
      return { mods: [...entries.values()], warnings };
    }, enabled: !!prefs.downloads,
    staleTime: 30_000 });
  const [platformFilter, setPlatformFilter] = useState<string>("target");
  const mods = platformFilter !== "target" ? (library.data?.mods ?? []).filter(m => platformFilter === "all" || m.platforms.includes(platformFilter)) : target ? (library.data?.mods ?? []).filter(m => matchesTarget(m, target)) : [];
  const mod = mods.find(m => selected && sameManagerPath(m.path, selected)) ?? mods.find(m => incoming?.buildPaths?.some(path => sameManagerPath(path, m.path))) ?? mods[0];
  const review = useQuery({ queryKey: ["mod-change-review", mod?.path, mod?.package_id],
    queryFn: async () => {
      const inspected = await call("package_inspect", { path: mod!.path });
      if (inspected.package_id !== mod!.package_id) throw new Error("This package changed. Refresh your mod library to review it.");
      return inspected;
    }, enabled: !!mod, staleTime: 30_000 });
  const installs = useQuery({ queryKey: ["package_installs"], queryFn: () => call("package_installs", {}), refetchOnMount: "always" });
  const recordsFor = (m: typeof mods[number]) => target ? installationRecords(installs.data ?? [], m, target) : [];
  const active = mod ? recordsFor(mod) : [];
  const [operationStage, setOperationStage] = useState("Checking installation…");
  const toggle = useMutation({ mutationFn: async ({ selectedMod, destination, deactivate }: { selectedMod: typeof mods[number]; destination: ManagerTarget; deactivate: boolean }) => {
    setOperationStage("Checking installation…");
    // Recheck persisted status at click time; no optimistic Installed badge after a failed write.
    const records = await call("package_installs", {});
    const current = installationRecords(records, selectedMod, destination);
    if (deactivate) {
      setOperationStage("Restoring original files…");
      for (const record of current) {
        const restored = await call("package_restore", { recordDir: record.record_dir });
        if (restored.state !== "restored") throw new Error(restored.detail || "Deactivation did not complete. Try again.");
      }
      return "Mod deactivated. Original files restored.";
    }
    if (current.some(r => r.recovery)) throw new Error("An earlier operation needs recovery. Deactivate this mod first.");
    if (current.length) return "Mod is already activated.";
    // Restore newest first, including any stacks created by earlier Studio versions.
    for (const record of targetInstallationRecords(records, destination)) {
      setOperationStage(`Deactivating ${record.package_name}…`);
      const restored = await call("package_restore", { recordDir: record.record_dir });
      if (restored.state !== "restored") throw new Error(restored.detail || "The current mod could not be deactivated. The new mod was not installed.");
    }
    setOperationStage("Preparing mod…");
    const plan = await call("package_preflight", { packagePath: selectedMod.path, gameId: destination.gameId,
      platform: destination.platform, root: destination.root, savePath: null, rosterSave: null, allowConflicts: false });
    if (plan.package_id !== selectedMod.package_id) throw new Error("This package has changed. Refresh your mod library and try again.");
    if (!plan.installable || plan.conflicts.length) throw new Error([...plan.reasons, ...plan.conflicts].join(" ") || "This mod cannot be activated for this installation.");
    setOperationStage("Installing mod…");
    const result = await call("package_apply", { handle: plan.handle });
    if (result.state !== "committed") throw new Error(result.detail || "Activation did not complete. Your installation status has been refreshed.");
    return "Mod activated.";
  }, onSettled: async () => { setOperationStage("Refreshing installation status…"); await qc.invalidateQueries({ queryKey: ["package_installs"] }); } });
  const busy = toggle.isPending;
  useEffect(() => { toggle.reset(); }, [mod?.path, target?.gameId, target?.platform, target?.root]);
  return <main className="flex h-screen flex-col bg-slate-950 text-slate-100">
    <header className="shrink-0 space-y-4 border-b border-slate-800 bg-slate-900/60 p-5">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-4"><img src={headerLogo} alt="NHL Modding Studio" className="h-9 w-auto" /><h1 className="text-lg font-semibold">Mod Manager</h1></div>
        <div className="flex gap-2"><Button disabled={busy} variant="secondary" onClick={() => setSetup("library")}>Mods folder</Button>{project ? <Button disabled={busy} variant="secondary" onClick={() => navigate("/app/database/pending")}>Return to Studio</Button> : <Button disabled={busy} variant="secondary" onClick={() => navigate("/")}>Main menu</Button>}</div>
      </div>
      {project && <SessionControls />}
      <div className="flex flex-wrap items-end gap-3">
        <label className="min-w-56 flex-1 text-xs text-slate-400">Installation target
          <select disabled={busy} className={fieldClass + " mt-1"} value={target ? targetKey(target) : ""} onChange={e => { savePreferences({ ...prefs, selected: e.target.value }); setSelected(null); }}>
            {!target && <option value="">Add a game to get started</option>}
            {prefs.targets.map(t => <option key={targetKey(t)} value={targetKey(t)}>{gameById(t.gameId)?.label} · {CONSOLE_LABEL[t.platform]}</option>)}
          </select>
        </label>
        {target && <GamePlatformPill gameId={target.gameId} platform={target.platform} />}
        <Button disabled={busy} variant="secondary" onClick={() => { setEditTarget(undefined); setSetup("target"); }}>Add game</Button>
        {target && <Button disabled={busy} variant="secondary" onClick={() => { setEditTarget(target); setSetup("target"); }}>Change installation folder</Button>}
      </div>
      {target && <p className="break-all text-xs text-slate-400">Installation folder: <span className="select-text text-slate-300">{target.root}</span></p>}
      {storageError && <p role="alert" className="text-xs text-amber-300">{storageError}</p>}
    </header>
    <div className="grid min-h-0 flex-1 grid-cols-[minmax(220px,28%)_minmax(0,1fr)]">
      <aside className="flex min-h-0 flex-col border-r border-slate-800 bg-slate-900/40" aria-label="Mod library">
        <div className="flex items-center justify-between gap-2 border-b border-slate-800 p-4"><h2 className="font-semibold">Mod library <span className="text-xs text-slate-400">({mods.length})</span></h2><Button variant="ghost" disabled={busy || library.isFetching || installs.isFetching || !prefs.downloads} onClick={() => { void library.refetch(); void installs.refetch(); void qc.invalidateQueries({ queryKey: ["mod-change-review"] }); }}>Refresh</Button></div>
        <label className="px-4 py-2 text-xs text-slate-400">Filter platforms
          <select aria-label="Mod platform filter" disabled={busy} className={fieldClass + " mt-1"} value={platformFilter} onChange={e => { setPlatformFilter(e.target.value); setSelected(null); }}>
            <option value="target">Selected game and platform</option><option value="all">All platforms</option>
            {PLATFORM_ORDER.map(p => <option key={p} value={p}>{CONSOLE_LABEL[p]}</option>)}
          </select>
        </label>
        <div className="min-h-0 flex-1 overflow-y-auto p-2">
          {library.isFetching && <p role="status" className="p-3 text-sm text-slate-400">Reading your mods…</p>}
          {library.isError && <p role="alert" className="break-words p-3 text-sm text-rose-300">Couldn’t read the mods folder. {String(library.error)}</p>}
          {!library.isFetching && !library.isError && !mods.length && <p className="p-3 text-sm text-slate-400">No mods for this game and platform. Add .nhlmod files to your mods folder, then refresh.</p>}
          {mods.map(m => <button disabled={busy} key={m.path} aria-pressed={m.path === mod?.path} onClick={() => setSelected(m.path)} className={"mb-1 flex w-full items-center gap-3 rounded-lg border p-2 text-left transition-colors " + (m.path === mod?.path ? "border-sky-700 bg-sky-950 text-sky-100" : "border-transparent hover:bg-slate-800")}>
            <img src={img(gameById(m.game_id)?.file ?? "")} alt={gameById(m.game_id)?.label ?? m.game_id} className="aspect-[320/176] h-14 shrink-0 rounded object-cover" />
            <span className="min-w-0">
              <span className="block break-words text-sm font-medium">{m.name}</span><span className="text-xs text-slate-400">Version {m.version}</span>
              {!installs.isError && recordsFor(m).some(r => r.state === "committed") && <span className="ml-2"><Pill tone="ok">Installed</Pill></span>}
            </span>
          </button>)}
        </div>
        <p className="shrink-0 break-all border-t border-slate-800 p-3 text-[11px] text-slate-500">{prefs.downloads}</p>
      </aside>
      <section aria-label="Mod details" className="min-w-0 overflow-y-auto p-6">
        {mod && !library.isError ? <div className="max-w-4xl space-y-5">
          <div className="flex items-start justify-between gap-4"><div><h2 className="text-2xl font-semibold">{mod.name}</h2><p className="mt-2 text-sm text-slate-400">Version {mod.version} · By {mod.author || "Unknown author"}</p></div>
            <Button disabled={busy || !target || !matchesTarget(mod, target) || !installs.isSuccess || installs.isFetching || !mod.package_id} onClick={() => toggle.mutate({ selectedMod: mod, destination: target, deactivate: active.length > 0 })}>{busy ? active.length ? "Deactivating…" : "Activating…" : active.length ? "Deactivate" : "Activate"}</Button>
          </div>
          {!installs.isError && active.some(r => r.state === "committed") && <Pill tone="ok">Installed</Pill>}
          {busy && <p role="status" className="text-sm text-sky-300">{operationStage}</p>}
          {!installs.isError && active.some(r => r.recovery) && <p className="text-sm text-amber-300">An earlier operation was interrupted. Deactivate to restore the original files.</p>}
          {installs.isError && <p role="alert" className="text-sm text-rose-300">Couldn’t check installation status. Click Refresh to try again.</p>}
          {toggle.isError && <p role="alert" className="text-sm text-rose-300">{String(toggle.error)}</p>}
          {toggle.isSuccess && <p role="status" className="text-sm text-emerald-300">{toggle.data}</p>}
          {!target || !matchesTarget(mod, target) ? <p className="text-sm text-amber-300">Select a matching installation target to activate this mod.</p> : null}
          {mod.platforms.map(p => <GamePlatformPill key={p} gameId={mod.game_id} platform={p as Platform} />)}
          <div className="whitespace-pre-wrap break-words rounded-xl border border-slate-800 bg-slate-900/50 p-5 text-sm leading-relaxed text-slate-300">{mod.description || "The author hasn’t provided a description."}</div>
          <section aria-label="Mod changes" className="space-y-3">
            <h3 className="font-semibold">Changes in this mod</h3>
            {review.isFetching && <p role="status" className="text-sm text-slate-400">Reading changes…</p>}
            {review.isError && <p role="alert" className="text-sm text-rose-300">Couldn’t read this mod’s changes. {String(review.error)} <Button variant="ghost" onClick={() => void review.refetch()}>Retry</Button></p>}
            {review.isSuccess && <ChangeTree key={mod.package_id} changes={review.data.changes ?? []} />}
          </section>
          <dl className="space-y-2 text-xs text-slate-400"><div><dt className="inline font-medium">Created: </dt><dd className="inline">{mod.created ? new Date(mod.created).toLocaleString() : "Not provided"}</dd></div><div><dt className="inline font-medium">Package: </dt><dd className="inline break-all select-text">{mod.path}</dd></div></dl>
        </div> : <p className="mt-10 text-center text-sm text-slate-400">Select a mod from your library to view its details.</p>}
        {!!library.data?.warnings.length && <details className="mt-6 rounded-lg border border-amber-800/60 p-3 text-xs text-amber-300"><summary className="cursor-pointer">{library.data.warnings.length} library warning(s)</summary>{library.data.warnings.map((w, i) => <p key={i} className="mt-2 break-words">{w}</p>)}</details>}
      </section>
    </div>
    {setup && <ManagerSetup key={`${setup}:${editTarget ? targetKey(editTarget) : ""}`} mode={setup} downloads={prefs.downloads} target={setup === "target" ? editTarget : undefined} suggestedGame={incoming?.gameId} suggestedPlatforms={incoming?.platforms}
      onCancel={() => { if (setup === "first") navigate("/"); else setSetup(null); }}
      onDone={(downloads, nextTarget) => {
        const targets = nextTarget ? [...prefs.targets.filter(t => targetKey(t) !== targetKey(nextTarget)), nextTarget] : prefs.targets;
        savePreferences({ downloads, targets, selected: nextTarget ? targetKey(nextTarget) : prefs.selected }); setSetup(null); setSelected(null);
      }} />}
  </main>;
}

function ManagerSetup({ mode, downloads, target, suggestedGame, suggestedPlatforms, onCancel, onDone }: { mode: SetupMode; downloads: string; target?: ManagerTarget; suggestedGame?: string; suggestedPlatforms?: string[]; onCancel: () => void; onDone: (downloads: string, target?: ManagerTarget) => void }) {
  const dialog = useRef<HTMLDialogElement>(null);
  const qc = useQueryClient();
  const [step, setStep] = useState(mode === "target" ? 1 : 0);
  const [folder, setFolder] = useState(downloads);
  const suggested = gameById(suggestedGame ?? "nhl07") ?? gameById("nhl07")!;
  const [gameId, setGameId] = useState(target?.gameId ?? suggested.id);
  const [platform, setPlatform] = useState<Platform>(target?.platform ?? suggested.platforms.find(p => suggestedPlatforms?.includes(p)) ?? suggested.platforms[0]);
  const [root, setRoot] = useState(target?.root ?? "");
  const [pickerError, setPickerError] = useState<string | null>(null);
  useEffect(() => { dialog.current?.showModal(); }, []);
  const next = useMutation({ mutationFn: async () => {
    if (step === 0) {
      const result = await call("mod_library_scan", { root: folder.trim() });
      qc.setQueryData(["mod-library", folder.trim()], result);
      if (mode === "library") onDone(folder.trim()); else setStep(1);
    } else {
      const verified = await call("mod_manager_check_target", { root: root.trim(), platform });
      onDone(folder.trim(), { gameId, platform, root: verified });
    }
  } });
  const browse = async () => {
    try { setPickerError(null); const path = await openDialog({ directory: true, multiple: false, title: step === 0 ? "Choose your downloaded mods folder" : "Choose the folder containing the game executable" }); if (typeof path === "string") { if (step === 0) setFolder(path); else setRoot(path); next.reset(); } }
    catch (error) { setPickerError(String(error)); }
  };
  return <dialog ref={dialog} aria-labelledby="manager-setup-title" onCancel={e => { e.preventDefault(); if (!next.isPending) onCancel(); }} className="fixed inset-0 m-auto w-[min(36rem,calc(100vw-2rem))] rounded-xl border border-slate-700 bg-slate-900 p-6 text-slate-100 backdrop:bg-black/70">
    <h2 id="manager-setup-title" className="text-lg font-semibold">{step === 0 ? "Your mod library" : "Choose your game installation"}</h2>
    {step === 0 ? <p className="my-4 text-sm text-slate-400">Choose the folder where you keep downloaded .nhlmod files. Mod Manager will include mods in its subfolders too.</p> : <div className="my-4 grid grid-cols-2 gap-3">
      <label className="text-xs text-slate-300">Game<select className={fieldClass + " mt-1"} disabled={next.isPending} value={gameId} onChange={e => { setGameId(e.target.value); setPlatform(gameById(e.target.value)!.platforms[0]); setRoot(""); next.reset(); }}>{GAMES.map(g => <option key={g.id} value={g.id}>{g.label}</option>)}</select></label>
      <label className="text-xs text-slate-300">Platform<select className={fieldClass + " mt-1"} disabled={next.isPending} value={platform} onChange={e => { setPlatform(e.target.value as Platform); setRoot(""); next.reset(); }}>{gameById(gameId)!.platforms.map(p => <option key={p} value={p}>{CONSOLE_LABEL[p]}</option>)}</select></label>
      <p className="col-span-2 text-sm text-slate-400">Choose the folder containing the game’s {platform === "xbox" ? ".xex" : platform === "recomp" ? ".exe or .elf" : ".bin or .elf"} file. This is where Mod Manager will install your mods.</p>
    </div>}
    <label className="block text-xs text-slate-300">{step === 0 ? "Downloaded mods folder" : "Game installation folder"}<input className={fieldClass + " mt-1"} disabled={next.isPending} value={step === 0 ? folder : root} onChange={e => { if (step === 0) setFolder(e.target.value); else setRoot(e.target.value); next.reset(); }} /></label>
    <Button variant="secondary" className="mt-2" disabled={next.isPending} onClick={() => void browse()}>Browse…</Button>
    {(next.isError || pickerError) && <p role="alert" className="mt-3 text-sm text-rose-300">{String(next.error ?? pickerError)}</p>}
    <div className="mt-6 flex justify-end gap-2"><Button variant="secondary" disabled={next.isPending} onClick={onCancel}>Cancel</Button>{mode === "first" && step === 1 && <Button variant="secondary" disabled={next.isPending} onClick={() => { setStep(0); next.reset(); }}>Back</Button>}<Button disabled={next.isPending || !(step === 0 ? folder : root).trim()} onClick={() => next.mutate()}>{next.isPending ? "Checking…" : step === 0 && mode !== "library" ? "Next" : "Finish"}</Button></div>
  </dialog>;
}

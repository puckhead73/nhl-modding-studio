import { useEffect, useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { useProject } from "../lib/project";
import { nhl24Open, nhl24Restore, nhl24Status } from "../lib/nhl24Roster";

const control = "rounded border border-slate-700 bg-slate-900 px-3 py-2 disabled:opacity-40";
const PATHS_KEY = "nhl24-roster-paths";

type Paths = { install: string; oodle: string };
function loadPaths(): Paths {
  try {
    return { install: "", oodle: "", ...JSON.parse(localStorage.getItem(PATHS_KEY) ?? "{}") };
  } catch {
    return { install: "", oodle: "" };
  }
}

export function Nhl24Roster() {
  const [paths, setPaths] = useState<Paths>(loadPaths);
  const { open } = useProject();
  const navigate = useNavigate();
  const qc = useQueryClient();
  useEffect(() => { try { localStorage.setItem(PATHS_KEY, JSON.stringify(paths)); } catch { /* per-viewer convenience only */ } }, [paths]);
  const status = useQuery({ queryKey: ["nhl24-roster-status", paths.install], queryFn: () => nhl24Status(paths.install), enabled: !!paths.install, retry: false });
  const opening = useMutation({ mutationFn: () => nhl24Open(paths.install, paths.oodle), onSuccess: o => {
    open(o.summary, o.db_path);
    navigate("/app/database/teams");
  } });
  const restoring = useMutation({ mutationFn: () => nhl24Restore(paths.install), onSuccess: () => qc.invalidateQueries({ queryKey: ["nhl24-roster-status"] }) });
  const choose = (key: keyof Paths) => async () => {
    const p = key === "install"
      ? await openDialog({ directory: true, multiple: false })
      : await openDialog({ multiple: false, filters: [{ name: "Oodle decoder", extensions: ["dll"] }] });
    if (typeof p === "string") setPaths(v => ({ ...v, [key]: p }));
  };
  const s = status.data;
  const error = opening.error ?? restoring.error ?? (paths.install ? status.error : null);
  return <main className="min-h-screen bg-slate-950 p-6 text-slate-100">
    <div className="mx-auto max-w-3xl space-y-4">
      <div className="flex justify-between">
        <Link to="/studio" className="text-sky-400">← Project picker</Link>
        <span className="space-x-4">
          <Link to="/frostbite" className="text-sky-400">Assets</Link>
          <Link to="/nhl24-tuning" className="text-sky-400">Gameplay tuning</Link>
        </span>
      </div>
      <h1 className="text-2xl font-semibold">NHL 24 rosters · PS4 01.60 (shadPS4)</h1>
      <p className="text-sm text-slate-400">Edit the game's roster database. Saving writes Studio's own files into the install's game-mods folder; the game's own files and other mods are never changed, and Restore removes Studio's files again. Close the game before saving.</p>
      <form className="grid gap-2 md:grid-cols-[1fr_auto]" onSubmit={e => { e.preventDefault(); opening.mutate(); }}>
        <input aria-label="NHL 24 folder" className={control} value={paths.install} placeholder="Folder holding game, game-UPDATE and game-mods" onChange={e => setPaths(v => ({ ...v, install: e.target.value }))} />
        <button type="button" className={control} onClick={choose("install")}>Browse</button>
        <input aria-label="Oodle decoder" className={control} value={paths.oodle} placeholder="oo2core_*_win64.dll (not included with Studio)" onChange={e => setPaths(v => ({ ...v, oodle: e.target.value }))} />
        <button type="button" className={control} onClick={choose("oodle")}>Browse</button>
        <button className={`${control} md:col-span-2`} disabled={!paths.install || !paths.oodle || opening.isPending}>{opening.isPending ? "Reading the roster…" : "Open roster"}</button>
      </form>
      {error && <p role="alert" className="rounded border border-red-800 p-3 text-red-300">{String(error)}</p>}
      {s && <section aria-label="Studio files in game-mods" className="space-y-2 rounded border border-slate-800 p-4 text-sm">
        {s.overlay_files.length === 0
          ? <p>The game loads its own roster database. Studio has no files in game-mods.</p>
          : <p>The game loads the roster Studio last saved ({s.overlay_files.length} Studio files in game-mods).</p>}
        {s.other_mod_files > 0 && <p className="text-slate-400">{s.other_mod_files} files from other mods are in game-mods; Studio leaves them alone.</p>}
        {s.emulator_running && <p className="text-amber-300">{s.emulator_running} is running. Close it before saving or restoring.</p>}
        {s.open_project != null && <p className="text-slate-400">This roster is open in Studio. Close it before restoring.</p>}
        {s.overlay_files.length > 0 && <button type="button" className={control} disabled={restoring.isPending || s.open_project != null} onClick={() => restoring.mutate()}>{restoring.isPending ? "Restoring…" : "Restore the game's own roster"}</button>}
      </section>}
    </div>
  </main>;
}

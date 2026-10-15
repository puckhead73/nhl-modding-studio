import { useEffect, useRef, useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { open } from "@tauri-apps/plugin-dialog";
import { call } from "../lib/client";
import { Button } from "./primitives";

export function ImportProjectArchive({ projectsRoot, disabled, onImported, pickOnMount = false }: { projectsRoot: string; disabled: boolean; onImported: (path: string) => void; pickOnMount?: boolean }) {
  const [archive, setArchive] = useState("");
  const [destination, setDestination] = useState("");
  const [source, setSource] = useState("");
  const [error, setError] = useState("");
  const [picking, setPicking] = useState(false);
  const imported = useMutation({ mutationFn: () => call("project_import_archive", { archive, destination, sourceRoot: source || null }), onSuccess: path => onImported(path) });
  const busy = disabled || picking || imported.isPending;
  const pickedInitially = useRef(false);
  useEffect(() => {
    if (pickOnMount && !disabled && !pickedInitially.current) {
      pickedInitially.current = true;
      void chooseArchive();
    }
  }, [pickOnMount, disabled]);
  async function chooseArchive() {
    setPicking(true); setError("");
    try {
      const path = await open({ multiple: false, filters: [{ name: "Studio developer project", extensions: ["nhlproject"] }] });
      if (typeof path !== "string") return;
      const info = await call("project_archive_preview", { archive: path });
      const name = (info.name || "Imported project").replace(/[<>:"/\\|?*]/g, "_").replace(/^[. ]+|[. ]+$/g, "") || "Imported project";
      setArchive(path); setDestination(`${projectsRoot.replace(/[\\/]$/, "")}/${info.platform === "xbox" ? "xbox360" : info.platform}/${info.game_id}/${name}`);
      imported.reset();
    } catch (e) { setError(String(e)); } finally { setPicking(false); }
  }
  async function browse(sourceFolder: boolean) {
    try {
      const folder = await open({ directory: true, multiple: false, title: sourceFolder ? "Choose game source folder" : "Choose parent folder for the imported project" });
      if (typeof folder !== "string") return;
      if (sourceFolder) setSource(folder);
      else setDestination(`${folder.replace(/[\\/]$/, "")}/${destination.split(/[\\/]/).pop() || "Imported project"}`);
    } catch (e) { setError(String(e)); }
  }
  return <section className="my-4 rounded-lg border border-slate-700 p-4">
    <Button variant="secondary" disabled={busy} onClick={() => void chooseArchive()}>Import developer project…</Button>
    {archive && <div className="mt-3 space-y-3">
      <p className="break-all text-xs text-slate-400">{archive}</p>
      <label className="block text-sm">New project folder<input aria-label="Imported project folder" className="mt-1 w-full rounded border border-slate-700 bg-slate-950 p-2" value={destination} disabled={busy} onChange={e => setDestination(e.target.value)} /></label>
      <Button variant="secondary" disabled={busy} onClick={() => void browse(false)}>Browse destination…</Button>
      <label className="block text-sm">Game source folder (optional)<input aria-label="Imported game source folder" className="mt-1 w-full rounded border border-slate-700 bg-slate-950 p-2" value={source} disabled={busy} onChange={e => setSource(e.target.value)} /></label>
      <Button variant="secondary" disabled={busy} onClick={() => void browse(true)}>Browse source…</Button>
      <p className="text-xs text-slate-400">Your saved edits come with the project. Link your own game folder for asset work, or add it later under Project Overview → Game source folders.</p>
      <Button disabled={busy || !destination.trim()} onClick={() => imported.mutate()}>{imported.isPending ? "Importing…" : "Import and open"}</Button>
    </div>}
    {(error || imported.isError) && <p role="alert" className="mt-2 text-sm text-rose-300">{error || String(imported.error)}</p>}
  </section>;
}

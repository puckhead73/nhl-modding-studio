import { useEffect, useRef, useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { call } from "../lib/client";
import type { InstallReportDto } from "../bindings/InstallReportDto";
import { ExtractionPanel, useExtractionJob, jobFinished, formatBytes } from "./ExtractionPanel";
import { Button, ErrorBanner } from "./primitives";

/** PS4 PKG files are materialized into a fresh game folder, then imported normally.
 * This route never offers disc write-back and never treats partial copies as ready. */
export function Ps4PackagePanel({ gameId, onReady, onBusyChange }: {
  gameId: string;
  onBusyChange?: (busy: boolean) => void;
  onReady: (report: InstallReportDto) => void;
}) {
  const [image, setImage] = useState("");
  const [extractor, setExtractor] = useState("");
  const [root, setRoot] = useState("");
  const [packedRoot, setPackedRoot] = useState<string | null>(null);
  const [archivesBusy, setArchivesBusy] = useState(false);
  const { job, setJob } = useExtractionJob(null);
  const mounted = useRef(true);
  const generation = useRef(0);
  useEffect(() => {
    mounted.current = true;
    return () => { mounted.current = false; generation.current++; };
  }, []);
  const start = useMutation({
    mutationFn: async () => {
      const gen = generation.current;
      const value = await call("pkg_extract_start", { image: image.trim(), root: root.trim(), gameId, extractor: extractor.trim() || undefined });
      return { gen, value };
    },
    onSuccess: ({ gen, value }) => { if (mounted.current && generation.current === gen) setJob(value); },
  });
  const cancel = useMutation({
    mutationFn: (id: number) => call("extract_cancel", { job: id }),
    onSuccess: (value) => { if (mounted.current) setJob(value); },
  });
  const useFiles = useMutation({
    mutationFn: async () => {
      if (job?.state !== "done") throw new Error("Complete package extraction first.");
      const gen = generation.current;
      const report = await call("detect_install", { gameId, platform: "ps4", root: job.root });
      if (!(report.game_dir || report.install_dir) || (!report.nhlng_path && !report.extract_needed)) {
        throw new Error("No readable game database was found. Use a supported PS4 base package or select your extracted files above.");
      }
      return { gen, report };
    },
    onSuccess: ({ gen, report }) => {
      if (!mounted.current || generation.current !== gen) return;
      // PS4 packages keep the database in game archives. Extract them before import.
      if (!report.nhlng_path) setPackedRoot(report.game_dir ?? report.install_dir);
      else { setPackedRoot(null); onReady(report); }
    },
  });
  const busy = archivesBusy || start.isPending || useFiles.isPending || (!!job && !jobFinished(job));
  useEffect(() => { onBusyChange?.(busy); return () => onBusyChange?.(false); }, [busy, onBusyChange]);
  function change(set: (value: string) => void, value: string) {
    generation.current++;
    set(value);
    setJob(null);
    setPackedRoot(null);
    start.reset(); useFiles.reset(); cancel.reset();
  }
  const cls = "w-full rounded-md border border-slate-700 bg-slate-950 px-2 py-1.5 text-sm text-slate-100";
  return <section className="mt-4 space-y-3 rounded-md border border-slate-800 p-3">
    <h3 className="text-sm font-semibold">Or extract a PS4 package image</h3>
    <p className="text-xs text-slate-400">Choose a base-game PKG and a new folder with an existing parent and enough free space.
      Studio prepares a separate game folder without changing the package. If its database is packed, extract the copied containers before importing.</p>
    <label className="block text-xs">PS4 base PKG
      <input className={cls} aria-label="PS4 base PKG" value={image} disabled={busy}
        onChange={(e) => change(setImage, e.target.value)} />
    </label>
    <Button disabled={busy} onClick={() => void (async () => {
      const gen = generation.current;
      const value = await openDialog({ multiple: false, directory: false, filters: [{ name: "PS4 PKG", extensions: ["pkg"] }] });
      if (mounted.current && generation.current === gen && typeof value === "string") change(setImage, value);
    })()}>Browse PKG…</Button>
    <label className="block text-xs">Standalone PS4 extractor (optional if included with Studio)
      <input className={cls} aria-label="PS4 package extractor" value={extractor} disabled={busy}
        placeholder="pkg_extractor.exe" onChange={e => change(setExtractor,e.target.value)} />
    </label>
    <p className="text-xs text-slate-400">Use the standalone shadPS4Plus package extractor with its DLLs beside it. Base games only; updates and DLC are not merged.</p>
    <Button disabled={busy} onClick={() => void (async () => {
      const gen = generation.current;
      const value = await openDialog({ multiple:false,directory:false,filters:[{name:"Package extractor",extensions:["exe"]}] });
      if (mounted.current && generation.current === gen && typeof value === "string") change(setExtractor,value);
    })()}>Browse extractor…</Button>
    <label className="block text-xs">New game files folder
      <input className={cls} aria-label="PS4 package output folder" value={root} disabled={busy}
        onChange={(e) => change(setRoot, e.target.value)} />
    </label>
    <div className="flex gap-2">
      <Button disabled={busy || !image.trim() || !root.trim() || job?.state === "done"}
        onClick={() => start.mutate()}>Extract PS4 package</Button>
      {job && !jobFinished(job) && <Button disabled={cancel.isPending || job.state === "cancelling"}
        onClick={() => cancel.mutate(job.job)}>Cancel extraction</Button>}
      <Button disabled={busy || job?.state !== "done"} onClick={() => useFiles.mutate()}>Use extracted files</Button>
    </div>
    {job && <p className="text-xs text-slate-400" role="status">{job.state} · {job.state === "done" ? `${formatBytes(job.bytes_written)} · ` : ""}{job.current}</p>}
    {packedRoot && <div className="space-y-2">
      <p className="text-xs text-amber-200">This package's database is inside a game container. Extract the containers below, then choose Use extracted files again.</p>
      <ExtractionPanel key={packedRoot} root={packedRoot} platform="ps4" mode="settings" onBusyChange={setArchivesBusy} />
    </div>}
    {(start.error || cancel.error || useFiles.error || job?.error) && <ErrorBanner title="Package extraction"
      message={String(start.error || cancel.error || useFiles.error || job?.error)} />}
  </section>;
}

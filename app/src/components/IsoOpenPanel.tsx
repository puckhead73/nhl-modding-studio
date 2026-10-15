import { useEffect, useRef, useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { call } from "../lib/client";
import type { Platform } from "../lib/client";
import type { IsoInspectDto } from "../bindings/IsoInspectDto";
import type { IsoStagedDto } from "../bindings/IsoStagedDto";
import type { ProjectSummaryDto } from "../bindings/ProjectSummaryDto";
import type { InstallReportDto } from "../bindings/InstallReportDto";
import { ExtractionPanel, useExtractionJob, jobFinished } from "./ExtractionPanel";
import { Button, ErrorBanner, Pill } from "./primitives";

const inputCls =
  "w-full rounded-md border border-slate-700 bg-slate-950 px-2 py-1.5 text-sm text-slate-100 placeholder-slate-600 focus:border-sky-600 focus:outline-none";

/// Open a title from its Xbox 360 disc image (`.iso`) instead of an extracted folder — the
/// source Xenia reads when it launches the image. Studio never edits the image directly: the
/// database and dictionary are read out into a staging folder (with `ISO-SOURCE.json` recording
/// where they came from and their identities), the ordinary workspace opens that folder, and
/// writing back to the image is a separate, explicit step on the Pending edits page.
///
/// Every async completion is bound to the request it started under (unmount / input change), so
/// a late inspection never describes a different image than the one in the field.
export function IsoOpenPanel({
  gameId,
  platform,
  defaultStagingRoot,
  onOpened,
}: {
  gameId: string;
  platform: Platform;
  /// A suggested staging folder for this title (the caller's saved-path entry or a default).
  defaultStagingRoot: string;
  onOpened: (staged: IsoStagedDto, summary: ProjectSummaryDto, install: InstallReportDto | null) => void;
}) {
  const [image, setImage] = useState("");
  const [staging, setStaging] = useState(defaultStagingRoot);
  const [inspection, setInspection] = useState<IsoInspectDto | null>(null);
  const [assets, setAssets] = useState("");
  const [archivesBusy, setArchivesBusy] = useState(false);
  const { job: discJob, setJob: setDiscJob } = useExtractionJob(null);
  const discBusy = !!discJob && !jobFinished(discJob);
  const cancelDisc = useMutation({
    mutationFn: (job: number) => call("extract_cancel", { job }),
    onSuccess: (job) => { if (mountedRef.current) setDiscJob(job); },
  });
  const extractDisc = useMutation({
    mutationFn: async () => {
      const gen = genRef.current;
      const job = await call("disc_extract_start", { image: image.trim(), root: assets.trim() });
      return { job, gen };
    },
    onSuccess: ({ job, gen }) => { if (mountedRef.current && gen === genRef.current) setDiscJob(job); },
  });
  const genRef = useRef(0);
  const mountedRef = useRef(true);
  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      genRef.current += 1;
    };
  }, []);
  const setImagePath = (p: string) => {
    genRef.current += 1;
    setInspection(null);
    setImage(p);
    setAssets("");
    setDiscJob(null);
  };

  const inspect = useMutation({
    mutationFn: async () => {
      const gen = genRef.current;
      const forImage = image.trim();
      const r = await call("iso_inspect", { image: forImage });
      return { r, gen, forImage };
    },
    onSuccess: ({ r, gen, forImage }) => {
      if (mountedRef.current && gen === genRef.current && forImage === image.trim()) setInspection(r);
    },
  });
  const openIso = useMutation({
    mutationFn: async () => {
      const gen = genRef.current;
      const install = assets.trim() ? await call("detect_install", { gameId, platform, root: assets.trim() }) : null;
      if (install && !install.install_dir) throw new Error("The game files folder was not recognized. Select the extracted disc's game folder.");
      const [staged, summary] = await call("iso_open", {
        image: image.trim(),
        gameId,
        platform,
        stagingRoot: staging.trim(),
        metaXml: null,
      });
      return { staged, summary, gen, install };
    },
    onSuccess: ({ staged, summary, gen, install }) => {
      // The project is open server-side either way; only a panel that still owns the request
      // publishes it to the shell (an unmounted or re-targeted panel leaves it to the shell's
      // lingering-project handling).
      if (mountedRef.current && gen === genRef.current) onOpened(staged, summary, install);
    },
  });

  async function browseImage() {
    const picked = await openDialog({
      multiple: false,
      directory: false,
      title: "Pick the disc image (.iso) or packed archive (.big)",
      filters: [{ name: "Xbox 360 disc image or EA archive", extensions: ["iso", "big"] }],
    });
    if (typeof picked === "string") setImagePath(picked);
  }
  async function browseStaging() {
    const picked = await openDialog({ multiple: false, directory: true, title: "Pick the staging folder" });
    if (typeof picked === "string") setStaging(picked);
  }

  const busy = inspect.isPending || openIso.isPending || extractDisc.isPending || discBusy || archivesBusy;
  const canOpen = !!image.trim() && !!staging.trim() && !busy && (!discJob || discJob.state === "done") && (inspection == null || inspection.nhlng != null);

  return (
    <div className="mt-5 rounded-md border border-slate-800 bg-slate-950/50 p-3">
      <div className="mb-2 flex items-center gap-2">
        <span className="text-xs font-semibold text-slate-200">Or open the disc image (.iso) or a packed archive (.big)</span>
        <Pill tone="warn">image route</Pill>
      </div>
      <p className="mb-3 text-[11px] text-slate-500">
        When Xenia launches the <code>.iso</code>, the extracted folder beside it is not what the game reads; when it
        launches the folder, the packed <code>cacheboot.big</code> / <code>cache.big</code> may win over the loose copies.
        Either way Studio copies the database and dictionary out of the container into a staging folder, you edit and
        save there, and writing back into the container is a separate explicit step (Pending edits → Disc image).
      </p>
      <div className="space-y-2">
        <div>
          <label className="mb-1 block text-xs font-medium text-slate-300">Disc image or archive file</label>
          <div className="flex gap-2">
            <input
              aria-label="disc image path"
              className={inputCls + " flex-1 font-mono text-xs"}
              value={image}
              disabled={discBusy || extractDisc.isPending || archivesBusy || openIso.isPending}
              onChange={(e) => setImagePath(e.target.value)}
              placeholder="X:\\…\\NHL 14 (USA, Europe).iso  or  X:\\…\\NHL 14\\cacheboot.big"
            />
            <Button variant="secondary" disabled={discBusy || extractDisc.isPending || archivesBusy || openIso.isPending} onClick={() => void browseImage()}>
              Browse…
            </Button>
            <Button variant="secondary" onClick={() => inspect.mutate()} disabled={!image.trim() || busy}>
              {inspect.isPending ? "Inspecting…" : "Inspect"}
            </Button>
          </div>
        </div>
        <div>
          <label className="mb-1 block text-xs font-medium text-slate-300">Staging folder (Studio edits here, never the image)</label>
          <div className="flex gap-2">
            <input
              aria-label="staging folder"
              className={inputCls + " flex-1 font-mono text-xs"}
              value={staging}
              disabled={openIso.isPending}
              onChange={(e) => setStaging(e.target.value)}
              placeholder="E:\\nhl-studio-staging\\nhl14-xbox360"
            />
            <Button variant="secondary" disabled={openIso.isPending} onClick={() => void browseStaging()}>
              Browse…
            </Button>
          </div>
        </div>
      </div>
      <div className="mt-3 space-y-2 border-t border-slate-800 pt-3">
        <label className="block text-xs text-slate-300">Game files folder (optional for database-only editing)</label>
        <input aria-label="ISO game files folder" className={inputCls} value={assets} disabled={busy}
          onChange={(e) => { genRef.current += 1; setAssets(e.target.value); setDiscJob(null); }}
          placeholder="Choose an existing extracted game folder, or enter a new folder for disc extraction" />
        <div className="flex gap-2">
          <Button variant="secondary" disabled={busy} onClick={async () => {
            const picked = await openDialog({ directory: true, multiple: false, title: "Choose existing game files" });
            if (typeof picked === "string") { genRef.current += 1; setAssets(picked); setDiscJob(null); }
          }}>Link existing folder…</Button>
          <Button variant="secondary" disabled={busy || !image.trim() || !assets.trim()} onClick={() => extractDisc.mutate()}>Extract full disc</Button>
          {discBusy && <Button variant="secondary" disabled={cancelDisc.isPending} onClick={() => cancelDisc.mutate(discJob!.job)}>Cancel disc extraction</Button>}
        </div>
        <p className="text-[11px] text-slate-500">Disc extraction needs a new folder with an existing parent and several GB of free space. Existing files are never overwritten. After copying, extract the game archives below before opening for asset editing. Asset edits target this copied tree; the Disc image write-back remains database/dictionary only.</p>
        {extractDisc.isError && <ErrorBanner title="Couldn't start disc extraction" message={String(extractDisc.error)} />}
        {cancelDisc.isError && <ErrorBanner title="Couldn't cancel disc extraction" message={String(cancelDisc.error)} />}
        {discJob && <p role="status" className="text-xs text-slate-300">{discJob.state}: {discJob.current}</p>}
        {discJob?.error && <ErrorBanner title="Disc copy incomplete" message={discJob.error} />}
        {discJob?.state === "done" && <ExtractionPanel root={discJob.root} platform={platform} mode="settings" onBusyChange={setArchivesBusy} />}
      </div>
      {inspect.isError && (
        <div className="mt-3">
          <ErrorBanner title="Couldn't read that image" message={String(inspect.error)} />
        </div>
      )}
      {inspection && (
        <div className="mt-3 space-y-1 text-xs text-slate-300" data-testid="iso-inspection">
          <p>
            <span className="text-slate-500">layout</span> {inspection.layout} ·{" "}
            <span className="text-slate-500">members</span> {inspection.members}
          </p>
          <p>
            <span className="text-slate-500">database</span>{" "}
            {inspection.nhlng ? (
              <>
                {inspection.nhlng.describe} · {inspection.nhlng_tables ?? "?"} tables ·{" "}
                <span className="font-mono">{inspection.nhlng_sha256?.slice(0, 12)}…</span>
              </>
            ) : (
              <span className="text-rose-300">not found in this image (probed {inspection.probed.length} archives)</span>
            )}
          </p>
          <p>
            <span className="text-slate-500">dictionary</span>{" "}
            {inspection.dictionary ? (
              <>
                {inspection.dictionary.describe} · {inspection.dictionary_rows ?? "?"} rows ·{" "}
                <span className="font-mono">{inspection.dictionary_sha256?.slice(0, 12)}…</span>
              </>
            ) : (
              <span className="text-amber-300">not found — labels will come from the vocabulary only</span>
            )}
          </p>
        </div>
      )}
      {openIso.isError && (
        <div className="mt-3">
          <ErrorBanner title="Couldn't open from the image" message={String(openIso.error)} />
        </div>
      )}
      <div className="mt-3 flex items-center gap-2">
        <Button onClick={() => openIso.mutate()} disabled={!canOpen}>
          {openIso.isPending ? "Staging and opening…" : "Stage and open"}
        </Button>
        <span className="text-[11px] text-slate-500">
          Reuses an existing staging of the same image (keeping its edits); refuses a folder used for another image.
        </span>
      </div>
    </div>
  );
}

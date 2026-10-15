// ONB-002 — the one extraction surface shared by onboarding's fourth screen and Settings → Tools:
// read-only preflight, the "Extract game assets now?" choice, a background job with progress and
// cancellation, and an honest result (written / retained / conflicting / blocked / unsupported).
// Extraction writes into the game installation on purpose (the explicit exception); it never
// touches the project folder and it is never a deployment.

import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { useEffect, useState } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import { call, type Platform } from "../lib/client";
import type { ExtractPreflightDto } from "../bindings/ExtractPreflightDto";
import type { ExtractJobDto } from "../bindings/ExtractJobDto";
import { Button, ErrorBanner, Spinner } from "./primitives";

export const EXTRACTION_PROMPT =
  "Prepare textures, models and audio for editing. You can do this later in Settings → Tools.";

export function formatBytes(n: number): string {
  if (n >= 1024 * 1024 * 1024) return `${(n / (1024 * 1024 * 1024)).toFixed(1)} GiB`;
  if (n >= 1024 * 1024) return `${(n / (1024 * 1024)).toFixed(0)} MiB`;
  if (n >= 1024) return `${(n / 1024).toFixed(0)} KiB`;
  return `${n} B`;
}

export function jobFinished(job: ExtractJobDto | null | undefined): boolean {
  return !!job && job.state !== "running" && job.state !== "cancelling";
}

/** Polls a job until it finishes. */
export function useExtractionJob(initial: ExtractJobDto | null) {
  const [job, setJob] = useState<ExtractJobDto | null>(initial);
  useEffect(() => {
    if (!job || jobFinished(job)) return;
    const t = setInterval(() => {
      call("extract_status", { job: job.job })
        .then(setJob)
        .catch(() => {});
    }, 500);
    return () => clearInterval(t);
  }, [job]);
  return { job, setJob };
}

export function ExtractionPanel({
  root,
  platform,
  mode,
  onSkip,
  onDone,
  onBusyChange,
  verificationPending = false,
  initialPreflight,
  projectId,
  initialDestination,
  onRelink,
}: {
  /** The resolved extraction destination (the game folder / install dir). */
  root: string;
  platform: Platform;
  /** `onboarding` shows the choice with Skip; `settings` is the Tools card. */
  mode: "onboarding" | "settings";
  onSkip?: () => void;
  /** Called with the finished job (done, cancelled or failed), or `null` when nothing needed doing. */
  onDone?: (job: ExtractJobDto | null, preflight: ExtractPreflightDto) => void;
  onBusyChange?: (busy: boolean) => void;
  verificationPending?: boolean;
  initialPreflight?: ExtractPreflightDto;
  projectId?: number;
  initialDestination?: string;
  onRelink?: (result: import("../bindings/ProjectRelinkDto").ProjectRelinkDto) => void;
}) {
  const [destination, setDestination] = useState(initialDestination ?? root);
  const [scanDestination, setScanDestination] = useState(initialDestination ?? root);
  useEffect(() => { const timer = setTimeout(() => setScanDestination(destination.trim()), 350); return () => clearTimeout(timer); }, [destination]);
  const [linking, setLinking] = useState(false);
  const [changeDestination, setChangeDestination] = useState(false);
  const [pathError, setPathError] = useState<string | null>(null);
  const preflight = useQuery({
    queryKey: ["extract_preflight", root, platform, scanDestination],
    queryFn: () => call("extract_preflight", { root, platform, destination: scanDestination }),
    enabled: root.trim() !== "",
    // Onboarding just scanned this source; reuse that result on the next screen.
    initialData: scanDestination === root ? initialPreflight : undefined,
    staleTime: initialPreflight ? 30_000 : 0,
  });
  // Attach to a job another screen started for the same destination.
  const running = useQuery({
    queryKey: ["extract_running_for", root],
    queryFn: () => call("extract_running_for", { root }),
    enabled: root.trim() !== "",
  });
  const { job, setJob } = useExtractionJob(null);
  useEffect(() => {
    if (running.data && !job) { if (running.data.destination) setDestination(running.data.destination); setJob(running.data); }
  }, [running.data, job, setJob]);

  const start = useMutation({
    mutationFn: () => call("extract_start", { root, platform, destination: destination.trim() }),
    onSuccess: job => { setPathError(null); setChangeDestination(false); setJob(job); },
  });
  const cancel = useMutation({
    mutationFn: () => call("extract_cancel", { job: job!.job }),
    onSuccess: setJob,
  });
  useEffect(() => {
    let current = true;
    if (job && jobFinished(job)) {
      // The wizard needs the post-extraction result, including unresolved collisions.
      setLinking(true);
      void (async () => {
        try {
          const result = await preflight.refetch();
          if (!current || !result.data || result.isError) return;
          if (projectId && job.state === "done") {
            const linked = await call("project_relink", { projectId, assetsDir: destination.trim() });
            if (!current) return;
            onRelink?.(linked);
          }
          if (current) onDone?.(job, result.data);
        } catch (error) { if (current) setPathError(`Couldn’t link extracted assets: ${String(error)}`); }
        finally { if (current) setLinking(false); }
      })();
    }
    return () => { current = false; };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [job?.job, job?.state, root, platform]);

  const pf = preflight.data;
  const unreadable = pf?.archives.filter(a => a.state === "unsupported" || a.state === "failed").length ?? 0;
  const remaining = pf?.archives.filter(a => a.state !== "complete" && a.state !== "unsupported" && a.state !== "failed").length ?? 0;
  const blockedReason = pf?.free_bytes != null && pf.bytes_to_write > pf.free_bytes
    ? "Not enough free space. Free up space and try again."
    : "Some files cannot be extracted safely. You can skip this step for now.";
  const busy = linking || (!!job && !jobFinished(job));
  const partial = !!job && (job.files_conflicting > 0 || job.files_blocked > 0 ||
    job.files_unsupported > 0 || job.files_unsafe > 0 || job.archives_failed > 0);
  useEffect(() => {
    onBusyChange?.(busy || start.isPending);
    return () => onBusyChange?.(false);
  }, [busy, start.isPending, onBusyChange]);

  const completed = !changeDestination && !busy && !pathError && ((job?.state === "done" && pf?.members_to_write === 0) || (!job && pf?.state === "complete"));
  if (completed) return <div role="status" data-testid="extraction-panel" className="rounded-md border border-emerald-800 bg-emerald-950/30 p-4">
    <p className="font-semibold text-emerald-300">Extraction completed</p>
    <p className="mt-1 text-sm text-slate-300">{partial ? "Extraction finished. Some files were skipped." : "Your game assets are ready for editing."}</p>
    <p className="mt-2 break-all text-xs text-slate-400">Assets folder: {destination}</p>
    <Button variant="secondary" className="mt-2" onClick={() => setChangeDestination(true)}>{partial ? "Review skipped files" : "Change assets folder"}</Button>
  </div>;

  return (
    <div className="rounded-md border border-slate-800 bg-slate-950/40 p-3" data-testid="extraction-panel">
      <div className="mb-1 flex items-center gap-2">
        <span className="text-sm font-semibold text-slate-100">
          {mode === "onboarding" ? "Extract game assets now?" : "Extract game containers"}
        </span>
      </div>
      <p className="text-xs text-slate-400">{EXTRACTION_PROMPT}</p>
      <label className="mt-3 block text-xs text-slate-300">Extract assets to
        <div className="mt-1 flex gap-2"><input aria-label="Extraction destination" disabled={busy || start.isPending} value={destination} onChange={e => setDestination(e.target.value)} className="min-w-0 flex-1 rounded border border-slate-700 bg-slate-950 p-2" />
          <Button variant="secondary" disabled={busy || start.isPending} onClick={() => { void openDialog({ directory: true, multiple: false, title: "Choose extracted assets folder" }).then(path => { if (typeof path === "string") setDestination(path); }).catch(error => setPathError(String(error))); }}>Browse…</Button></div>
      </label>
      {pathError && <p role="alert" className="text-xs text-rose-300">{pathError}</p>}
      {preflight.isLoading && <Spinner label="Checking game assets…" />}
      {preflight.isError && <ErrorBanner title="Couldn't check game assets" message="Try again, or skip this step for now." />}
      {pf && (
        <div className="mt-2 text-xs text-slate-300" data-testid="extraction-preflight">
          <p>{remaining > 0 ? `${remaining} container${remaining === 1 ? " needs" : "s need"} extracting.` : unreadable > 0 ? "No other containers need extracting." : "No extraction needed."}</p>
          {pf.state === "blocked" && <p className="mt-1 text-amber-300">{blockedReason}</p>}
          {pf.collisions.length > 0 && (
            <div className="mt-1 text-amber-300">
              Some files conflict and will be skipped.
            </div>
          )}
          {unreadable > 0 && (
            <div className="mt-1 text-amber-300">
              {unreadable} container{unreadable === 1 ? "" : "s"} could not be read and will be skipped.
            </div>
          )}
        </div>
      )}

      {job && (
        <div className="mt-3 text-xs" data-testid="extraction-job">
          <div className="flex items-center gap-2">
            {!jobFinished(job) && <Spinner />}
            <span className="font-medium text-slate-100">
              {job.state === "running" && "Extracting…"}
              {job.state === "cancelling" && "Stopping…"}
              {job.state === "done" && (partial ? "Extraction finished. Some files were skipped." : "Extraction finished")}
              {job.state === "cancelled" && "Extraction paused. You can resume later."}
              {job.state === "failed" && "Extraction stopped"}
            </span>
          </div>
          <div className="mt-1 text-slate-400">
            {job.archives_done} of {job.archives_total} containers processed
          </div>
          {busy && <div className="mt-3 space-y-3">
            {job.current && <p className="truncate text-slate-400">{job.current.split(/[\\/]/).pop()}</p>}
            <ExtractionProgress label="Current container" done={job.current_files_done ?? 0} total={job.current_files_total ?? 0} />
            <ExtractionProgress label="Overall extraction" done={job.files_done ?? 0} total={job.files_total ?? 0} />
          </div>}
          {job.error && <p className="mt-1 text-rose-300">Couldn't finish extraction. You can try again or continue for now.</p>}
        </div>
      )}

      <div className="mt-3 flex flex-wrap items-center gap-2">
        {busy ? (
          <Button variant="secondary" onClick={() => cancel.mutate()} disabled={cancel.isPending || job?.state === "cancelling"}>
            Cancel
          </Button>
        ) : (
          <Button
            onClick={() => start.mutate()}
            disabled={!destination.trim() || destination.trim() !== scanDestination || verificationPending || !pf || pf.state === "not_applicable" || (pf.state === "complete" && destination === root) || pf.state === "blocked" || start.isPending}
            title={pf?.state === "complete" || pf?.state === "not_applicable" ? "No extraction needed" : pf?.state === "blocked" ? blockedReason : undefined}
          >
            {job?.state === "cancelled" ? "Extract remaining" : "Extract now"}
          </Button>
        )}
        {mode === "onboarding" && onSkip && (
          <Button variant="secondary" onClick={onSkip} disabled={busy || verificationPending}>
            Skip for now
          </Button>
        )}
        {start.isError && <span className="text-xs text-rose-300">{String(start.error)}</span>}
      </div>
    </div>
  );
}

function ExtractionProgress({ label, done, total }: { label: string; done: number; total: number }) {
  return <div><div className="mb-1 flex justify-between text-slate-300"><span>{label}</span><span>{total ? `${done.toLocaleString()} / ${total.toLocaleString()} files` : "Preparing…"}</span></div>
    <progress aria-label={label} className="h-2 w-full accent-sky-500" max={total || 1} value={total ? Math.min(done, total) : undefined} />
  </div>;
}

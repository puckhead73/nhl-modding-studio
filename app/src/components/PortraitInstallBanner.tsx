// Progress strip for the background portrait-database download / install. Sits under the primary
// tabs in the Studio shell (and on the onboarding card); shows only while a job runs or has just
// finished, and hides itself once the operator dismisses the outcome.
import { useEffect, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { formatBytes, isActivePhase, usePortraitInstallActions, usePortraitInstallStatus } from "../lib/portraitInstall";

export function PortraitInstallBanner({ compact = false }: { compact?: boolean }) {
  const status = usePortraitInstallStatus();
  const { cancel } = usePortraitInstallActions();
  const qc = useQueryClient();
  const [dismissedAt, setDismissedAt] = useState<string | null>(null);
  const s = status.data;
  // A finished job also refreshes the library status the historical pages show.
  useEffect(() => {
    if (s?.phase === "done") void qc.invalidateQueries({ queryKey: ["portraitLibrary"] });
  }, [s?.phase, qc]);
  if (!s || s.phase === "idle") return null;
  const active = isActivePhase(s.phase);
  if (!active && dismissedAt === (s.finished_at ?? s.phase)) return null;

  const pct =
    s.phase === "downloading" && s.bytes_total
      ? Math.min(100, (s.bytes_done / s.bytes_total) * 100)
      : null;
  const tone =
    s.phase === "failed" ? "border-rose-800 bg-rose-950/50 text-rose-100"
    : s.phase === "done" ? "border-emerald-800 bg-emerald-950/40 text-emerald-100"
    : s.phase === "cancelled" ? "border-slate-700 bg-slate-900 text-slate-300"
    : "border-sky-900 bg-sky-950/40 text-sky-100";
  const label =
    s.phase === "downloading"
      ? `Downloading the portrait database — ${formatBytes(s.bytes_done)}${s.bytes_total ? ` of ${formatBytes(s.bytes_total)}` : ""}${pct != null ? ` (${pct.toFixed(1)}%)` : ""}`
      : s.phase === "extracting"
        ? `Installing the portrait database — ${s.files_done.toLocaleString()} files extracted`
        : s.message;

  return (
    <div
      role="status"
      data-testid="portrait-install-banner"
      className={`flex shrink-0 items-center gap-3 border-b px-4 ${compact ? "py-1" : "py-1.5"} text-[11px] ${tone}`}
    >
      <div className="min-w-0 flex-1">
        <div className="truncate">{label}</div>
        {active && (
          <div className="mt-1 h-1.5 w-full overflow-hidden rounded bg-slate-800">
            <div
              className={`h-full rounded ${pct == null ? "w-1/3 animate-pulse bg-sky-500/70" : "bg-sky-500"}`}
              style={pct != null ? { width: `${pct}%` } : undefined}
            />
          </div>
        )}
        {s.phase === "failed" && s.error && !compact && (
          <div className="mt-0.5 truncate text-rose-200/80" title={s.error}>
            Retry from Settings → General → Portrait database; a stopped download resumes where it left off.
          </div>
        )}
      </div>
      {active ? (
        <button
          type="button"
          onClick={() => cancel.mutate()}
          disabled={cancel.isPending}
          className="rounded border border-current/40 px-2 py-0.5 text-[11px] hover:bg-white/5 disabled:opacity-50"
        >
          Cancel
        </button>
      ) : (
        <button
          type="button"
          onClick={() => setDismissedAt(s.finished_at ?? s.phase)}
          className="rounded border border-current/40 px-2 py-0.5 text-[11px] hover:bg-white/5"
        >
          Dismiss
        </button>
      )}
    </div>
  );
}

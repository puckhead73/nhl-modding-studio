import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { call } from "../lib/client";

/// The app-level "Legends ratings owed" banner.
///
/// Legends ranks players on ratings precomputed per formula profile, so any profile that has never
/// been calculated (or was calculated against a different corpus build) leaves the builder unusable.
/// Declining the calculation at save time parks that debt here rather than losing it: the flag lives
/// in `ea_formula_state.json` beside the corpus, so it survives closing the app and returns on the
/// next launch until the user either runs the calculation or dismisses it.
///
/// Dismissing is an acknowledgement, not a fix — Legends stays unavailable until it's run.
export function LegendsPendingBanner() {
  const qc = useQueryClient();
  const status = useQuery({
    queryKey: ["legends_status"],
    queryFn: () => call("legends_status", {}),
    // No corpus open → no Legends debt to report. Failing quietly keeps this out of the way of
    // users who never touch the historical side.
    retry: false,
  });

  const compute = useMutation({
    mutationFn: () => call("compute_legends_ratings", {}),
    onSuccess: async () => {
      await qc.invalidateQueries({ queryKey: ["legends_status"] });
      await qc.invalidateQueries({ queryKey: ["legends_teams"] });
    },
  });
  const dismiss = useMutation({
    mutationFn: () => call("set_legends_pending", { pending: true, dismissed: true }),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["legends_status"] }),
  });

  const s = status.data;
  if (!s || !s.pending || s.dismissed) return null;

  const busy = compute.isPending;
  return (
    <div className="flex shrink-0 flex-wrap items-center gap-2 border-b border-amber-900/60 bg-amber-950/40 px-4 py-1.5 text-xs text-amber-200">
      <span aria-hidden>🏒</span>
      <span>
        {busy ? (
          <>
            Calculating Legends ratings for <strong>{s.active_profile}</strong> — this reads every
            player's every season, so it takes a while.
          </>
        ) : s.stale ? (
          <>
            The Legends ratings for <strong>{s.active_profile}</strong> were built against a
            different corpus, so they can't be trusted. Recalculate to use the Legends builder.
          </>
        ) : (
          <>
            Legends ratings haven't been calculated for <strong>{s.active_profile}</strong>. The
            Legends builder can't rank players until they are.
          </>
        )}
      </span>
      <div className="ml-auto flex items-center gap-2">
        <button
          type="button"
          disabled={busy}
          onClick={() => compute.mutate()}
          className="rounded-md border border-amber-500/40 bg-amber-600 px-3 py-1 text-xs font-medium text-white transition-colors hover:bg-amber-500 disabled:cursor-not-allowed disabled:opacity-50"
        >
          {busy ? "Calculating…" : "Calculate now"}
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={() => dismiss.mutate()}
          title="Hide this until you switch or save a profile. Legends stays unavailable until you calculate."
          className="rounded-md border border-amber-800/70 px-2.5 py-1 text-xs text-amber-200/80 hover:bg-amber-900/40 disabled:opacity-50"
        >
          Not now
        </button>
      </div>
      {compute.isError && (
        <span className="w-full text-rose-300">{String(compute.error)}</span>
      )}
      {compute.isSuccess && (
        <span className="w-full text-amber-200/70">
          Rated {compute.data.rated.toLocaleString()} of {compute.data.examined.toLocaleString()}{" "}
          players in {(compute.data.elapsed_ms / 1000).toFixed(1)}s.
        </span>
      )}
    </div>
  );
}

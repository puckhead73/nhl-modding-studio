import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { EmptyState, Spinner } from "./primitives";
import type { DraftPickDto } from "../bindings/DraftPickDto";

const ORDINAL = ["", "1st", "2nd", "3rd", "4th", "5th", "6th", "7th"];
const roundLabel = (round: number) => ORDINAL[round] ?? `R${round}`;

/// The Draft Picks tab — every future pick the team owns (`qHav`), grouped by draft year, with
/// trade provenance (origin team resolved server-side per the human-readable-surfaces rule).
export function DraftPicksTab({ projectId, teamId }: { projectId: number; teamId: number }) {
  const picksQ = useQuery({
    queryKey: ["draft-picks", projectId, teamId],
    queryFn: () => call("team_draft_picks", { projectId, teamId }),
  });

  if (picksQ.isLoading) return <Spinner label="Loading draft picks…" />;
  if (picksQ.isError) return <p className="text-rose-400">{String(picksQ.error)}</p>;
  const picks = picksQ.data ?? [];
  if (picks.length === 0) {
    return <EmptyState title="This team owns no future draft picks" />;
  }

  const byYear = new Map<number, DraftPickDto[]>();
  for (const p of picks) {
    byYear.set(p.year, [...(byYear.get(p.year) ?? []), p]);
  }

  return (
    <div className="space-y-4">
      <div className="flex items-baseline justify-between gap-3">
        <h2 className="text-sm font-semibold text-slate-100">Draft Picks</h2>
        <span className="text-xs text-slate-400">
          {picks.length} future {picks.length === 1 ? "pick" : "picks"} owned
        </span>
      </div>
      {[...byYear.entries()]
        .sort((a, b) => a[0] - b[0])
        .map(([year, yearPicks]) => (
          <section key={year} className="overflow-hidden rounded-lg border border-slate-800">
            <header className="border-b border-slate-800 bg-slate-900/60 px-3 py-1.5 text-xs font-semibold text-slate-200">
              {year} Draft · {yearPicks.length} {yearPicks.length === 1 ? "pick" : "picks"}
            </header>
            {/* Fixed column widths so Round / Overall / Origin line up vertically across every
                year section (each year is its own table — auto layout would size them apart). */}
            <table className="w-full table-fixed text-left text-xs">
              <thead className="border-b border-slate-800 text-[10px] uppercase tracking-wider text-slate-500">
                <tr>
                  <th className="w-36 px-3 py-1.5 font-medium">Round</th>
                  <th className="w-24 px-2 py-1.5 text-right font-medium">Overall</th>
                  <th className="px-3 py-1.5 font-medium">Origin</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-900">
                {yearPicks.map((p) => (
                  <tr key={`${p.round}-${p.overall}`} className="hover:bg-slate-800/40">
                    <td className="px-3 py-1.5 text-slate-100">{roundLabel(p.round)} round</td>
                    <td className="px-2 py-1.5 text-right font-mono tabular-nums text-slate-300">
                      #{p.overall}
                    </td>
                    <td className="px-3 py-1.5 text-slate-400">
                      {p.via_trade ? (
                        <span className="flex flex-wrap items-center gap-1.5">
                          <span className="rounded-full border border-sky-800 bg-sky-900/40 px-2 py-0.5 text-[10px] text-sky-300">
                            via trade
                          </span>
                          <span>from {p.original_owner_name ?? "—"}</span>
                        </span>
                      ) : (
                        <span className="text-slate-500">Own pick</span>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </section>
        ))}
    </div>
  );
}

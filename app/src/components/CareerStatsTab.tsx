import { useEffect, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { clampRaw, friendlySeed, friendlyToRaw } from "../lib/editForm";
import { EmptyState, Spinner } from "./primitives";
import type { CareerStatsDto } from "../bindings/CareerStatsDto";
import type { CareerSectionDto } from "../bindings/CareerSectionDto";
import type { CareerRowDto } from "../bindings/CareerRowDto";
import type { FieldViewDto } from "../bindings/FieldViewDto";
import type { PlayerViewDto } from "../bindings/PlayerViewDto";

/// The Career Stats tab — the four stock career-stats sections (skater/goalie × regular
/// season/playoffs) in the reference layout: Year | Lg | stat columns, newest season first. (The
/// stored team code still identifies a row — it's set in Add Season — it just isn't a column.) Stat cells edit
/// inline through the audited `set_field` path (offset/scale-aware); seasons can be added (identity
/// set at add time, stats typed in afterwards) and deleted (ordered shift). The player's own kind
/// always shows both its sections; the other kind only when it actually has rows.
export function CareerStatsTab({
  projectId,
  player,
  career,
  loading,
}: {
  projectId: number;
  player: PlayerViewDto;
  career: CareerStatsDto | undefined;
  loading: boolean;
}) {
  if (loading) return <Spinner label="Loading…" />;
  if (!career) return <p className="text-sm text-slate-400">No career data.</p>;

  const goalie = player.kind === "goalie";
  const mine = (k: string) => k.startsWith(goalie ? "goalie" : "skater");
  const ordered = [...career.sections].sort(
    (a, b) => Number(mine(b.kind)) - Number(mine(a.kind)),
  );
  const visible = ordered.filter((s) => mine(s.kind) || s.rows.length > 0);
  const total = career.sections.reduce((n, s) => n + s.rows.length, 0);

  return (
    <div className="space-y-5">
      {total === 0 && (
        <EmptyState
          title="No stock career rows for this player"
          hint="Created players start without career data — use Add Season on a section below."
        />
      )}
      {visible.map((s) => (
        <Section key={s.kind} projectId={projectId} playerId={player.player_id} section={s} />
      ))}
    </div>
  );
}

function Section({
  projectId,
  playerId,
  section,
}: {
  projectId: number;
  playerId: number;
  section: CareerSectionDto;
}) {
  const [adding, setAdding] = useState(false);
  return (
    // No title bar — the table stands on its own (its title rides on the tooltip); Add Season
    // lives in a slim footer instead.
    <section className="rounded-lg border border-slate-800 bg-slate-900/40" title={section.title}>
      {adding && (
        <AddSeasonForm
          projectId={projectId}
          playerId={playerId}
          section={section}
          onDone={() => setAdding(false)}
        />
      )}

      {section.rows.length === 0 ? (
        <p className="px-3 py-3 text-sm text-slate-500">No rows.</p>
      ) : (
        <div className="overflow-x-auto p-2">
          <table className="w-full border-collapse text-xs">
            <thead>
              <tr className="text-left font-semibold uppercase tracking-wider text-slate-400">
                <th className="px-2 py-1">Year</th>
                <th className="px-2 py-1">Lg</th>
                {section.columns.map((c) => (
                  <th key={c.field} className="px-2 py-1 text-right" title={c.field}>
                    {c.label}
                  </th>
                ))}
                <th className="w-8 px-2 py-1" />
              </tr>
            </thead>
            <tbody>
              {section.rows.map((row) => (
                <Row key={row.record} projectId={projectId} section={section} row={row} />
              ))}
            </tbody>
          </table>
        </div>
      )}

      <div className="flex justify-end border-t border-slate-800/60 px-2 py-1">
        <button
          type="button"
          onClick={() => setAdding((v) => !v)}
          className="rounded px-2 py-0.5 text-xs text-slate-500 hover:bg-slate-800 hover:text-sky-300"
        >
          {adding ? "Cancel" : "+ Add Season"}
        </button>
      </div>
    </section>
  );
}

function Row({
  projectId,
  section,
  row,
}: {
  projectId: number;
  section: CareerSectionDto;
  row: CareerRowDto;
}) {
  return (
    <tr className="border-t border-slate-800/60 text-slate-200">
      <td className="whitespace-nowrap px-2 py-1 font-mono text-[11px] text-slate-100">
        {row.season_label}
      </td>
      <td className="px-2 py-1 text-slate-400">{row.league_label}</td>
      {row.cells.map((cell, i) => (
        <td key={section.columns[i].field} className="px-1 py-0.5 text-right">
          {cell ? (
            <StatCell projectId={projectId} table={section.table} record={row.record} field={cell} />
          ) : (
            <span className="text-slate-600">—</span>
          )}
        </td>
      ))}
      <td className="px-1 py-0.5 text-right">
        <DeleteRowButton projectId={projectId} table={section.table} record={row.record} />
      </td>
    </tr>
  );
}

/// A compact inline stat editor: renders as plain text, becomes an input on focus, and stages the
/// (offset/scale-aware) value through `set_field` on commit. Read-only cells just display.
function StatCell({
  projectId,
  table,
  record,
  field,
}: {
  projectId: number;
  table: string;
  record: number;
  field: FieldViewDto;
}) {
  const qc = useQueryClient();
  const { update } = useProject();
  const mut = useMutation({
    mutationFn: (intValue: number) =>
      call("set_field", { projectId, table, record, field: field.studio_field, intValue }),
    onSuccess: async () => {
      await qc.invalidateQueries();
      update(await call("project_summary", { projectId }));
    },
  });

  const baseline = friendlySeed(field);
  const [draft, setDraft] = useState(baseline);
  useEffect(() => setDraft(baseline), [baseline]);

  if (!field.editable) {
    return <span className="px-1 font-mono text-xs text-slate-300">{field.display}</span>;
  }

  const commit = () => {
    if (draft === baseline) return;
    const raw = friendlyToRaw(field, draft);
    if (raw == null) {
      setDraft(baseline);
      return;
    }
    mut.mutate(clampRaw(field, raw));
  };

  return (
    <span title={mut.error != null ? String(mut.error) : field.name}>
      <input
        type="text"
        inputMode="decimal"
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => {
          if (e.key === "Enter") commit();
          if (e.key === "Escape") setDraft(baseline);
        }}
        className={
          "w-12 rounded border border-transparent bg-transparent px-1 py-0.5 text-right font-mono text-xs " +
          "text-slate-100 hover:border-slate-700 focus:border-sky-600 focus:bg-slate-950 focus:outline-none " +
          (mut.error != null ? "border-rose-500 text-rose-300" : "") +
          (mut.isPending ? " opacity-50" : "")
        }
      />
    </span>
  );
}

/// Two-click delete: first click arms, second confirms. Ordered removal (later rows slide up).
function DeleteRowButton({
  projectId,
  table,
  record,
}: {
  projectId: number;
  table: string;
  record: number;
}) {
  const qc = useQueryClient();
  const { update } = useProject();
  const [armed, setArmed] = useState(false);
  const mut = useMutation({
    mutationFn: () => call("delete_career_row", { projectId, table, record }),
    onSuccess: async () => {
      await qc.invalidateQueries();
      update(await call("project_summary", { projectId }));
    },
  });
  return (
    <button
      type="button"
      title={armed ? "Click again to delete this season row" : "Delete season row"}
      onClick={() => (armed ? mut.mutate() : setArmed(true))}
      onBlur={() => setArmed(false)}
      disabled={mut.isPending}
      className={
        "rounded px-1.5 py-0.5 text-xs " +
        (armed
          ? "bg-rose-600 text-white"
          : "text-slate-500 hover:bg-slate-800 hover:text-rose-400")
      }
    >
      {armed ? "sure?" : "✕"}
    </button>
  );
}

/// The Add Season form: season-end year, league, and team code — the row's identity. The new row's
/// stat cells start at zero and are edited in place.
function AddSeasonForm({
  projectId,
  playerId,
  section,
  onDone,
}: {
  projectId: number;
  playerId: number;
  section: CareerSectionDto;
  onDone: () => void;
}) {
  const qc = useQueryClient();
  const { update } = useProject();
  const newestYear = section.rows[0] ? Number(section.rows[0].season_label.slice(0, 4)) + 2 : 2016;
  const [year, setYear] = useState(String(newestYear));
  const [league, setLeague] = useState(0);
  const [team, setTeam] = useState("");
  const mut = useMutation({
    mutationFn: () =>
      call("add_career_season", {
        projectId,
        table: section.table,
        playerId,
        yearEnd: Number(year),
        league,
        team: team.trim(),
      }),
    onSuccess: async () => {
      await qc.invalidateQueries();
      update(await call("project_summary", { projectId }));
      onDone();
    },
  });

  const yearOk = /^\d{4}$/.test(year.trim()) && Number(year) >= 1901 && Number(year) <= 2155;
  const teamOk = team.trim().length > 0 && team.trim().length <= 12;

  return (
    <div className="flex flex-wrap items-end gap-3 border-b border-slate-800 bg-slate-900/60 px-3 py-2">
      <label className="text-xs text-slate-400">
        Season end
        <input
          type="number"
          min={1901}
          max={2155}
          value={year}
          onChange={(e) => setYear(e.target.value)}
          className="mt-0.5 block w-24 rounded border border-slate-700 bg-slate-950 px-2 py-1 text-sm text-slate-100 focus:border-sky-600 focus:outline-none"
        />
      </label>
      <label className="text-xs text-slate-400">
        League
        <select
          value={league}
          onChange={(e) => setLeague(Number(e.target.value))}
          className="mt-0.5 block rounded border border-slate-700 bg-slate-950 px-2 py-1 text-sm text-slate-100 focus:border-sky-600 focus:outline-none"
        >
          <option value={0}>NHL</option>
          <option value={1}>AHL</option>
        </select>
      </label>
      <label className="text-xs text-slate-400">
        Team code
        <input
          type="text"
          maxLength={12}
          placeholder="e.g. PIT"
          value={team}
          onChange={(e) => setTeam(e.target.value)}
          className="mt-0.5 block w-28 rounded border border-slate-700 bg-slate-950 px-2 py-1 text-sm text-slate-100 focus:border-sky-600 focus:outline-none"
        />
      </label>
      <button
        type="button"
        disabled={!yearOk || !teamOk || mut.isPending}
        onClick={() => mut.mutate()}
        className="rounded bg-sky-700 px-3 py-1 text-sm font-medium text-white hover:bg-sky-600 disabled:opacity-40"
      >
        {mut.isPending ? "Adding…" : "Add"}
      </button>
      {typeof year === "string" && year !== "" && yearOk && (
        <span className="pb-1 text-xs text-slate-500">
          = the {Number(year) - 1}-{String(Number(year) % 100).padStart(2, "0")} season
        </span>
      )}
      {mut.error != null && (
        <span className="pb-1 text-xs text-rose-400">{String(mut.error)}</span>
      )}
    </div>
  );
}

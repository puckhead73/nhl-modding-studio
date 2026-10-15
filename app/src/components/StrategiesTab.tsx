import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router-dom";
import { call } from "../lib/client";
import { useFieldEdit } from "../lib/useFieldEdit";
import { EmptyState, Spinner } from "./primitives";
import { EnumSelect } from "./EnumSelect";
import {
  BIAS_MAX,
  BIAS_MIN,
  DEFENSE_BIAS_AXES,
  FORWARD_BIAS_AXES,
  strategyFieldHasGifs,
  strategyGifUrl,
  TACTIC_SLIDERS,
  type BiasAxis,
} from "../lib/teamPresentation";
import { POS_BADGE } from "../lib/position";
import type { LineEntryDto } from "../bindings/LineEntryDto";
import type { LinesViewDto } from "../bindings/LinesViewDto";
import type { PlayerSummaryDto } from "../bindings/PlayerSummaryDto";
import type { StrategyCellDto } from "../bindings/StrategyCellDto";
import type { TeamStrategiesDto } from "../bindings/TeamStrategiesDto";

/// The Strategies tab — the team's `EeYo` strategy sheet in the reference layout's three
/// operator-defined sections: Team Strategies (tactic dropdowns + the PP Carry/Dump dial, each
/// with its rink-diagram gif), Forward Lines' Strategies (4 lines × style + 4 named bias
/// sliders), and Defensive Pairing Strategies (3 pairs × 2 named sliders). Unmapped biases
/// (forward 5/6, defense 3..6, team 2..6) stay backend-readable but are not surfaced — they have
/// no operator-confirmed semantics yet. Every edit goes through the audited `set_field` path,
/// and the backend mirrors it onto the career-mode table (`JUfQ`) in the same transaction.
export function StrategiesTab({ projectId, teamId }: { projectId: number; teamId: number }) {
  const q = useQuery({
    queryKey: ["strategies", projectId, teamId],
    queryFn: () => call("team_strategies", { projectId, teamId }),
  });
  // The Lines assignments — read-only reference so the operator can see who they're setting
  // strategy for. Shares the Lines tab's query cache; the player summaries supply archetypes.
  const lines = useQuery({
    queryKey: ["lines", projectId, teamId],
    queryFn: () => call("lines_view", { projectId, teamId }),
  });
  const players = useQuery({
    queryKey: ["players", projectId],
    queryFn: () => call("list_players", { projectId }),
  });
  const summaryOf = new Map((players.data ?? []).map((p) => [p.player_id, p]));

  if (q.isLoading) return <Spinner label="Loading strategies…" />;
  if (q.isError) return <p className="text-rose-400">{String(q.error)}</p>;
  const view = q.data;
  if (!view) {
    return (
      <EmptyState
        title="No strategy sheet"
        hint="This database has no exhibitionteamstrategies (EeYo) row for the team."
      />
    );
  }

  return (
    <div className="space-y-4">
      <TeamStrategiesSection projectId={projectId} view={view} />
      <ForwardLinesSection
        projectId={projectId}
        view={view}
        lines={lines.data ?? null}
        summaryOf={summaryOf}
      />
      <DefensivePairingsSection
        projectId={projectId}
        view={view}
        lines={lines.data ?? null}
        summaryOf={summaryOf}
      />
      <p className="text-[11px] text-slate-500">
        Line assignments are read-only here — set them on the Lines tab. Edits apply to the
        exhibition sheet (EeYo) and are mirrored to the career-mode table (JUfQ) when a dynasty
        save has populated it.
      </p>
    </div>
  );
}

/** Occupant lookup by slot metadata name (`l1c`, `l2rd`, …) from the Lines view. */
function slotOccupant(lines: LinesViewDto | null, name: string): LineEntryDto | undefined {
  if (!lines) return undefined;
  const id = lines.slot_fields.find((s) => s.name === name)?.studio_field;
  if (!id) return undefined;
  return lines.entries.find((e) => e.slots.includes(id));
}

/** Section heading — the Player Data tab's section style. */
function LineSectionHeading({ children }: { children: React.ReactNode }) {
  return (
    <h4 className="mb-3 border-b border-slate-800 pb-1 text-[11px] font-bold uppercase tracking-wider text-slate-400">
      {children}
    </h4>
  );
}

/** The tone class for a slot's position badge — the app-wide position badge palette, with the
 *  handed defense/wing slots (`LD`/`RD`) taking their base position's tone. */
function slotBadgeTone(pos: string): string {
  const base = pos === "LD" || pos === "RD" ? "D" : pos;
  return POS_BADGE[base] ?? "bg-slate-800 text-slate-300";
}

/** The read-only "who's on this line" row shown across the top of each line section — one card
 *  per slot, in on-ice order, with the player's archetype under their name. Assignments come
 *  from the Lines view; archetypes from the player summaries. */
function LinePlayersRow({
  lines,
  slots,
  summaryOf,
}: {
  lines: LinesViewDto | null;
  /** `(position label, slot metadata name)` pairs, in display order. */
  slots: Array<[string, string]>;
  summaryOf: Map<number, PlayerSummaryDto>;
}) {
  return (
    <div className="mb-3 flex flex-wrap gap-2" title="Assigned on the Lines tab">
      {slots.map(([pos, name]) => {
        const occupant = slotOccupant(lines, name);
        const archetype =
          occupant?.player_id != null
            ? summaryOf.get(occupant.player_id)?.style_label?.trim()
            : undefined;
        return (
          <div
            key={name}
            className="flex min-w-[10rem] flex-1 items-start gap-2 rounded-md border border-slate-800 bg-slate-950/40 px-2 py-1 text-xs"
          >
            <span
              className={
                "mt-px inline-flex shrink-0 items-center rounded px-1.5 py-0.5 text-xs font-semibold " +
                slotBadgeTone(pos)
              }
            >
              {pos}
            </span>
            {occupant?.player_id != null ? (
              <span className="flex min-w-0 flex-1 flex-col">
                <span className="flex items-baseline gap-1.5">
                  {occupant.jersey != null && occupant.jersey > 0 && (
                    <span className="shrink-0 font-mono text-[10px] tabular-nums text-slate-500">
                      #{occupant.jersey}
                    </span>
                  )}
                  <Link
                    to={`/app/database/players/${occupant.player_id}`}
                    className="min-w-0 truncate text-slate-200 hover:text-sky-300"
                  >
                    {occupant.name || `Player #${occupant.player_id}`}
                  </Link>
                </span>
                {archetype && (
                  <span className="truncate text-[10px] text-slate-500">{archetype}</span>
                )}
              </span>
            ) : (
              <span className="self-center italic text-slate-600">vacant</span>
            )}
          </div>
        );
      })}
    </div>
  );
}

function findCell(view: TeamStrategiesDto, fieldId: string): StrategyCellDto | undefined {
  return (
    view.tactics.find((c) => c.field.studio_field === fieldId) ??
    view.forward_biases.find((c) => c.field.studio_field === fieldId) ??
    view.defense_biases.find((c) => c.field.studio_field === fieldId) ??
    view.team_biases.find((c) => c.field.studio_field === fieldId)
  );
}

// Built-in tactic vocabularies (the in-game strategy menus, raw value = menu index) — the same
// labels as `defs/enums/strategy-*.toml`. The registry labels win when the backend delivers
// them; this map guarantees the dropdowns render even against a registry that predates the
// strategy defs, exactly as the reference app carried them client-side.
const TACTIC_VOCABS: Record<string, readonly string[]> = {
  RBGh: ["Weak Side Lock", "2-3", "1-2-2 Passive", "1-2-2 Aggressive"],
  uAeC: ["1-3-1", "1-4", "1-2-2 Red Line", "1-2-2 Blue Line"],
  rKer: ["Collapsing", "Staggered", "Tight Point"],
  bame: ["Shooting", "Overload", "Umbrella"],
  NzPy: ["Passive Box", "Large Box", "Diamond"],
  TTVS: ["Defend Lead", "Conservative", "Standard", "Aggressive", "Full Attack"],
  xKvv: ["Protect Net", "Contain Puck", "Normal", "Puck Side Attack", "High Pressure"],
};
for (const id of ["eRIF", "LIlj", "ALBZ", "CPwO"]) {
  TACTIC_VOCABS[id] = ["Overload (rush offense)", "Crash The Net", "Behind The Net"];
}

/** The dropdown vocabulary for a tactic cell: registry-delivered options when present, the
 *  built-in vocabulary otherwise, plus an explicit entry for an out-of-vocab stored value so
 *  the select never renders blank. */
function tacticOptions(cell: StrategyCellDto): Array<{ raw: number; label: string }> {
  const fromRegistry = cell.field.enum_options.map((o) => ({ raw: o.raw, label: o.label }));
  const options =
    fromRegistry.length > 0
      ? fromRegistry
      : (TACTIC_VOCABS[cell.field.studio_field] ?? []).map((label, raw) => ({ raw, label }));
  const current = cell.field.decoded;
  if (current != null && !options.some((o) => o.raw === current)) {
    options.push({ raw: current, label: `(uncatalogued: ${current})` });
  }
  return options;
}

// ---------------------------------------------------------------------------
// Section: Team Strategies (tactic grid, each with its diagram)
// ---------------------------------------------------------------------------

const TEAM_ITEMS: Array<{ fieldId: string; label: string }> = [
  { fieldId: "RBGh", label: "Forecheck Formation" },
  { fieldId: "uAeC", label: "Neutral Zone Formation" },
  { fieldId: "asrd", label: "Neutral Zone Pressure" },
  { fieldId: "TTVS", label: "Offensive Pressure" },
  { fieldId: "xKvv", label: "Defensive Pressure" },
  { fieldId: "rKer", label: "Defensive Strategy" },
  { fieldId: "bame", label: "Power Play Strategy" },
  { fieldId: "NzPy", label: "Penalty Kill Strategy" },
  { fieldId: "JWjc", label: "Power Play Carry / Dump" },
];

function TeamStrategiesSection({
  projectId,
  view,
}: {
  projectId: number;
  view: TeamStrategiesDto;
}) {
  return (
    <section className="rounded-lg border border-slate-800 bg-slate-900/60 p-4">
      <h3 className="mb-3 text-[11px] font-bold uppercase tracking-wider text-slate-400">
        Team Strategies
      </h3>
      <div className="grid grid-cols-1 gap-x-6 gap-y-4 sm:grid-cols-2 lg:grid-cols-3">
        {TEAM_ITEMS.map((item) => {
          const cell = findCell(view, item.fieldId);
          if (!cell) return null;
          return (
            <div key={item.fieldId} className="flex flex-col gap-1">
              <span className="text-[10px] uppercase tracking-wider text-slate-500">
                {item.label}
              </span>
              {strategyFieldHasGifs(item.fieldId) && (
                <StrategyGif cell={cell} sizeClass="h-32 w-full" />
              )}
              <StrategyControl projectId={projectId} record={view.record} cell={cell} />
            </div>
          );
        })}
      </div>
    </section>
  );
}

// ---------------------------------------------------------------------------
// Section: Forward Lines' Strategies (4 lines × style + 4 bias sliders)
// ---------------------------------------------------------------------------

const LINE_STYLE_FIELDS = ["eRIF", "LIlj", "ALBZ", "CPwO"] as const;

function ForwardLinesSection({
  projectId,
  view,
  lines,
  summaryOf,
}: {
  projectId: number;
  view: TeamStrategiesDto;
  lines: LinesViewDto | null;
  summaryOf: Map<number, PlayerSummaryDto>;
}) {
  // Backend emits forward biases line-major: 4 lines × 6 biases.
  const matrix = Array.from({ length: 4 }, (_, r) => view.forward_biases.slice(r * 6, r * 6 + 6));
  return (
    <section className="rounded-lg border border-slate-800 bg-slate-900/60 p-4">
      <h3 className="mb-3 text-[11px] font-bold uppercase tracking-wider text-slate-400">
        Forward Lines' Strategies
      </h3>
      <div className="space-y-5">
        {matrix.map((biasCells, r) => {
          const n = r + 1;
          const styleCell = findCell(view, LINE_STYLE_FIELDS[r]);
          return (
            <section key={r}>
              <LineSectionHeading>Offense Line {n}</LineSectionHeading>
              <LinePlayersRow
                lines={lines}
                summaryOf={summaryOf}
                slots={[
                  ["LW", `l${n}lw`],
                  ["C", `l${n}c`],
                  ["RW", `l${n}rw`],
                ]}
              />
              <div className="flex flex-wrap items-start gap-4">
                {styleCell && (
                  <div className="flex w-56 shrink-0 flex-col gap-1">
                    <StrategyGif cell={styleCell} sizeClass="h-32 w-56" />
                    <StrategyControl projectId={projectId} record={view.record} cell={styleCell} />
                  </div>
                )}
                <div className="min-w-[14rem] flex-1 space-y-2 self-center">
                  {FORWARD_BIAS_AXES.map((axis) => {
                    const c = biasCells[axis.index];
                    return (
                      c && (
                        <BiasSlider
                          key={axis.label}
                          projectId={projectId}
                          record={view.record}
                          cell={c}
                          axis={axis}
                          showLabel
                        />
                      )
                    );
                  })}
                </div>
              </div>
            </section>
          );
        })}
      </div>
    </section>
  );
}

// ---------------------------------------------------------------------------
// Section: Defensive Pairing Strategies (3 pairs × 2 sliders)
// ---------------------------------------------------------------------------

function DefensivePairingsSection({
  projectId,
  view,
  lines,
  summaryOf,
}: {
  projectId: number;
  view: TeamStrategiesDto;
  lines: LinesViewDto | null;
  summaryOf: Map<number, PlayerSummaryDto>;
}) {
  const matrix = Array.from({ length: 3 }, (_, r) => view.defense_biases.slice(r * 6, r * 6 + 6));
  return (
    <section className="rounded-lg border border-slate-800 bg-slate-900/60 p-4">
      <h3 className="mb-3 text-[11px] font-bold uppercase tracking-wider text-slate-400">
        Defensive Pairing Strategies
      </h3>
      <div className="space-y-5">
        {matrix.map((biasCells, r) => {
          const n = r + 1;
          return (
            <section key={r}>
              <LineSectionHeading>Defense Pair {n}</LineSectionHeading>
              <LinePlayersRow
                lines={lines}
                summaryOf={summaryOf}
                slots={[
                  ["LD", `l${n}ld`],
                  ["RD", `l${n}rd`],
                ]}
              />
              <div className="max-w-md space-y-2">
                {DEFENSE_BIAS_AXES.map((axis) => {
                  const c = biasCells[axis.index];
                  return (
                    c && (
                      <BiasSlider
                        key={axis.label}
                        projectId={projectId}
                        record={view.record}
                        cell={c}
                        axis={axis}
                        showLabel
                      />
                    )
                  );
                })}
              </div>
            </section>
          );
        })}
      </div>
    </section>
  );
}

// ---------------------------------------------------------------------------
// Cell widgets
// ---------------------------------------------------------------------------

/** Rink-diagram gif for the cell's current value; a same-size placeholder when the value has no
 *  shipped diagram so neighbouring layout doesn't jump. */
function StrategyGif({ cell, sizeClass }: { cell: StrategyCellDto; sizeClass: string }) {
  const raw = cell.field.decoded ?? 0;
  const url = strategyGifUrl(cell.field.studio_field, raw);
  const frame =
    "flex items-center justify-center overflow-hidden rounded border border-slate-800 bg-slate-950/60";
  if (!url) {
    return (
      <div className={`${sizeClass} ${frame}`} title={`No diagram shipped for value ${raw}.`}>
        <span className="text-[10px] uppercase tracking-wider text-slate-600">no diagram</span>
      </div>
    );
  }
  return (
    <div className={`${sizeClass} ${frame}`}>
      <img
        src={url}
        alt={`${cell.label} diagram`}
        className="max-h-full max-w-full object-contain"
      />
    </div>
  );
}

/** Dropdown (tactics), named slider (NZ Pressure / PP Carry-Dump), or numeric fallback. */
function StrategyControl({
  projectId,
  record,
  cell,
}: {
  projectId: number;
  record: number;
  cell: StrategyCellDto;
}) {
  const mut = useFieldEdit(projectId, record, cell.field);
  const f = mut.field!;
  const slider = TACTIC_SLIDERS[f.studio_field];

  if (slider && f.enum_options.length === 0) {
    return (
      <BiasSlider
        projectId={projectId}
        record={record}
        cell={cell}
        axis={{ index: 0, label: cell.label, lowLabel: slider.lowLabel, highLabel: slider.highLabel }}
        min={slider.min}
        max={slider.max}
      />
    );
  }
  const options = tacticOptions(cell);
  if (options.length > 0) {
    return (
      <div title={mut.error != null ? String(mut.error) : `${f.studio_table}.${f.studio_field}`}>
        <select
          value={f.decoded ?? 0}
          disabled={!f.editable || mut.isPending}
          onChange={(e) => mut.mutate({ intValue: Number(e.target.value) })}
          className="w-full rounded border border-slate-700 bg-slate-950 px-1.5 py-0.5 font-mono text-xs text-slate-100 focus:border-sky-600 focus:outline-none"
        >
          {options.map((o) => (
            <option key={o.raw} value={o.raw}>
              {o.label}
            </option>
          ))}
        </select>
      </div>
    );
  }
  return <span className="font-mono text-xs text-slate-100">{f.display}</span>;
}

/** Bias slider with named endpoints. Storage is 5-bit but the in-game UI exposes 0..8 (NZ
 *  Pressure exposes its full 3-bit 0..7). Commits on release, not per-tick. `showLabel` names
 *  the axis above the slider (used in the vertically-stacked layouts, which have no table
 *  header row to carry it). */
function BiasSlider({
  projectId,
  record,
  cell,
  axis,
  min = BIAS_MIN,
  max = BIAS_MAX,
  showLabel = false,
}: {
  projectId: number;
  record: number;
  cell: StrategyCellDto;
  axis: BiasAxis;
  min?: number;
  max?: number;
  showLabel?: boolean;
}) {
  const mut = useFieldEdit(projectId, record, cell.field);
  const stored = mut.field?.decoded ?? 0;
  const [draft, setDraft] = useState(stored);
  useEffect(() => setDraft(stored), [stored]);

  const commit = () => {
    if (draft !== stored) mut.mutate({ intValue: draft });
  };

  // Named choices take precedence over a numeric bias slider when the title supplies them.
  if (cell.field.enum_options.length > 0) {
    return <div title={mut.error != null ? String(mut.error) : cell.label}>
      {showLabel && <div className="text-[10px] uppercase tracking-wide text-slate-500">{axis.label}</div>}
      <EnumSelect label={cell.label} value={stored} options={cell.field.enum_options}
        disabled={!cell.field.editable || mut.isPending}
        onChange={(raw) => mut.mutate({ intValue: raw })} />
    </div>;
  }

  return (
    <div
      className={"space-y-0.5 " + (mut.isPending ? "opacity-60" : "")}
      title={
        mut.error != null
          ? String(mut.error)
          : `${cell.label} — EeYo.${cell.field.studio_field}, stored ${stored}`
      }
    >
      {showLabel && (
        <div className="text-[10px] uppercase tracking-wider text-slate-500">{axis.label}</div>
      )}
      <input
        type="range"
        min={min}
        max={max}
        step={1}
        value={Math.min(draft, max)}
        disabled={!cell.field.editable || mut.isPending}
        onChange={(e) => setDraft(Number(e.target.value))}
        onPointerUp={commit}
        onKeyUp={(e) => {
          if (e.key === "ArrowLeft" || e.key === "ArrowRight") commit();
        }}
        className="w-full accent-sky-600"
      />
      <div className="flex justify-between text-[9px] uppercase tracking-wide text-slate-500">
        <span>{axis.lowLabel}</span>
        <span className="font-mono text-[10px] text-slate-300">{draft}</span>
        <span>{axis.highLabel}</span>
      </div>
    </div>
  );
}

import { useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useRosterEdit } from "../lib/useRosterEdit";
import { pendingField } from "../lib/useFieldEdit";
import { EmptyState, Spinner } from "./primitives";
import { ratingToneClass } from "../lib/ratingColor";
import { isDressed, styleCode } from "../lib/teamPresentation";
import type { LineEntryDto } from "../bindings/LineEntryDto";
import type { FieldViewDto } from "../bindings/FieldViewDto";
import type { LinesViewDto } from "../bindings/LinesViewDto";
import type { PlayerSummaryDto } from "../bindings/PlayerSummaryDto";

// ---------------------------------------------------------------------------
// The Lines tab — the reference editor: game-situation buckets (Even Strength / Power Play /
// Penalty Kill / 4-on-4 OT / Shootout / Extra Attacker) as sub-tabs, each rendering its units as
// rows of player cards; a draggable roster rail on the right with per-row status + archetype
// controls; drag-and-drop slot assignment (card ↔ card swaps, rail → card placement); and a
// "Best lines" auto-filler (Overall-ranked by position).
//
// The whole layout derives from the `eGlu` slot-field catalog the backend exposes — the metadata
// names (`l1c`, `pp4_2ld`, `ot_3rd`, `s1`, `x2`, …) encode group + position, so every bucket
// below is built by name lookup and titles/units adapt to whatever this title's schema carries.
// Slot edits are 1-bit `set_field` writes on the occupant rows (audited + undoable).
// ---------------------------------------------------------------------------

const LINES_DRAG_MIME = "text/plain"; // Tauri's webview drops custom MIME types on dragover.

interface DragPayload {
  slotRecord: number;
  sourceSlotId: string | null;
}

interface SlotSpec {
  /** Studio field id, present only when this title's schema carries the slot. */
  id?: string;
  position: string;
}

interface UnitSpec {
  title: string;
  /** Forwards-first, then defense — mirrors the reference layout split. */
  slots: SlotSpec[];
  forwardCount: number;
}

interface BucketSpec {
  key: string;
  title: string;
  units: UnitSpec[];
}

/** Build the situational layout from the slot-field catalog (name → studio id). */
function buildBuckets(view: LinesViewDto): BucketSpec[] {
  const byName = new Map(view.slot_fields.map((s) => [s.name, s.studio_field]));
  const slot = (name: string, position: string): SlotSpec => ({ id: byName.get(name), position });
  const present = (u: UnitSpec) => u.slots.some((s) => s.id !== undefined);

  const evenStrength: UnitSpec[] = [
    ...[1, 2, 3, 4].map((n) => ({
      title: `Offense Line ${n}`,
      slots: [slot(`l${n}lw`, "LW"), slot(`l${n}c`, "C"), slot(`l${n}rw`, "RW")],
      forwardCount: 3,
    })),
    ...[1, 2, 3].map((n) => ({
      title: `Defense Pair ${n}`,
      slots: [slot(`l${n}ld`, "LD"), slot(`l${n}rd`, "RD")],
      forwardCount: 2,
    })),
    {
      title: "Goalies",
      slots: [slot("g1", "Starter"), slot("g2", "Backup")],
      forwardCount: 2,
    },
  ];
  const unit = (title: string, names: string[], positions: string[], fc: number): UnitSpec => ({
    title,
    slots: names.map((n, i) => slot(n, positions[i])),
    forwardCount: fc,
  });
  const powerPlay = [
    ...[1, 2].map((n) =>
      unit(
        `Power Play 5v4 · Unit ${n}`,
        [`pp${n}c`, `pp${n}lw`, `pp${n}rw`, `pp${n}ld`, `pp${n}rd`],
        ["C", "LW", "RW", "LD", "RD"],
        3,
      ),
    ),
    ...[1, 2].map((n) =>
      unit(
        `Power Play 4v3 · Unit ${n}`,
        [`pp4_${n}c`, `pp4_${n}lw`, `pp4_${n}ld`, `pp4_${n}rd`],
        ["C", "LW", "LD", "RD"],
        2,
      ),
    ),
  ];
  const penaltyKill = [
    ...[1, 2].map((n) =>
      unit(
        `Penalty Kill 4v5 · Unit ${n}`,
        [`pk4_${n}c`, `pk4_${n}lw`, `pk4_${n}ld`, `pk4_${n}rd`],
        ["C", "LW", "LD", "RD"],
        2,
      ),
    ),
    ...[1, 2].map((n) =>
      unit(
        `Penalty Kill 3v5 · Unit ${n}`,
        [`pk3_${n}c`, `pk3_${n}ld`, `pk3_${n}rd`],
        ["C", "LD", "RD"],
        1,
      ),
    ),
  ];
  const overtime = [1, 2, 3].map((n) =>
    unit(
      `Overtime (4-on-4) · Unit ${n}`,
      [`ot_${n}c`, `ot_${n}lw`, `ot_${n}ld`, `ot_${n}rd`],
      ["C", "LW", "LD", "RD"],
      2,
    ),
  );
  const shootout = [
    unit("Shootout Order", ["s1", "s2", "s3", "s4", "s5"], ["#1", "#2", "#3", "#4", "#5"], 5),
  ];
  const extraAttacker = [unit("Extra Attacker (6v5)", ["x1", "x2"], ["#1", "#2"], 2)];

  const buckets: BucketSpec[] = [
    { key: "even", title: "Even Strength", units: evenStrength },
    { key: "pp", title: "Power Play", units: powerPlay },
    { key: "pk", title: "Penalty Kill", units: penaltyKill },
    { key: "ot", title: "4-on-4 / Overtime", units: overtime },
    { key: "so", title: "Shootout", units: shootout },
    { key: "x", title: "Extra Attacker", units: extraAttacker },
  ];
  return buckets
    .map((b) => ({ ...b, units: b.units.filter(present) }))
    .filter((b) => b.units.length > 0);
}

const SCRATCHED_TAB = "__scratched__";

export function LinesTab({ projectId, teamId }: { projectId: number; teamId: number }) {
  const lines = useQuery({
    queryKey: ["lines", projectId, teamId],
    queryFn: () => call("lines_view", { projectId, teamId }),
  });
  const players = useQuery({
    queryKey: ["players", projectId],
    queryFn: () => call("list_players", { projectId }),
  });
  const [activeBucket, setActiveBucket] = useState<string | null>(null);
  const [dragOverSlot, setDragOverSlot] = useState<string | null>(null);

  const summaryOf = useMemo(() => {
    const m = new Map<number, PlayerSummaryDto>();
    for (const p of players.data ?? []) m.set(p.player_id, p);
    return m;
  }, [players.data]);

  type SlotWrite = { record: number; field: string; value: 0 | 1 };
  const slotsMut = useRosterEdit(projectId, teamId,
    (writes: SlotWrite[]) => call("set_team_roster", { projectId, team: teamId, writes: writes.map(w => [w.record, w.field, w.value]) }),
    (before, writes) => ({ ...before, entries: before.entries.map(e => {
      const slots = new Set(e.slots);
      for (const w of writes) if (w.record === e.slot_record) { if (w.value) slots.add(w.field); else slots.delete(w.field); }
      return { ...e, slots: [...slots] };
    }) }));
  const applySlotWrites = (writes: SlotWrite[]) => { if (writes.length) slotsMut.mutate(writes); };
  const statusMut = useRosterEdit(projectId, teamId,
    (v: { entry: LineEntryDto; dressed: boolean }) => call("set_dressed", { projectId, team: teamId, player: v.entry.player_id!, dressed: v.dressed }),
    (before, v) => ({ ...before, entries: before.entries.map(e => e.slot_record === v.entry.slot_record
      ? { ...e, dressed: v.dressed, slots: v.dressed ? e.slots : [], captaincy: v.dressed ? e.captaincy : 0 } : e) }));
  const archetypeMut = useRosterEdit(projectId, teamId,
    (v: { record: number; value: number }) => call("set_field", { projectId, table: "eGlu", record: v.record, field: "QgFs", intValue: v.value }),
    (before, v) => ({ ...before, entries: before.entries.map(e => e.slot_record === v.record && e.style
      ? { ...e, style: pendingField(e.style, { intValue: v.value }) } : e) }));
  const busy = slotsMut.isPending;
  const mutationError = slotsMut.error ?? statusMut.error ?? archetypeMut.error;

  if (lines.isLoading || players.isLoading) return <Spinner label="Loading lines…" />;
  if (lines.isError) return <p className="text-rose-400">{String(lines.error)}</p>;
  const view = lines.data;
  if (!view) return <EmptyState title="No line data for this team" />;

  const entries = view.entries;
  const entryByRecord = new Map(entries.map((e) => [e.slot_record, e]));
  // Slot studio id → occupant eGlu record (rows whose bit is set).
  const slotOwner = new Map<string, number>();
  for (const e of entries) for (const f of e.slots) slotOwner.set(f, e.slot_record);

  const buckets = buildBuckets(view);
  const scratched = entries.filter((e) => e.dressed === false);
  const tabKeys = [...buckets.map((b) => b.key), SCRATCHED_TAB];
  const selectedKey = activeBucket && tabKeys.includes(activeBucket) ? activeBucket : tabKeys[0];

  const totalSlots = view.slot_fields.length;
  const totalAssigned = slotOwner.size;

  const dropToSlot = (payload: DragPayload, targetSlotId: string) => {
    const occupantRecord = slotOwner.get(targetSlotId);
    const { slotRecord: sourceRecord, sourceSlotId } = payload;
    if (sourceSlotId === targetSlotId) return;
    if (occupantRecord === sourceRecord && sourceSlotId === null) return;
    const writes: Array<{ record: number; field: string; value: 0 | 1 }> = [];
    if (sourceSlotId !== null) {
      // Card → card: a true swap.
      writes.push({ record: sourceRecord, field: sourceSlotId, value: 0 });
      writes.push({ record: sourceRecord, field: targetSlotId, value: 1 });
      if (occupantRecord !== undefined && occupantRecord !== sourceRecord) {
        writes.push({ record: occupantRecord, field: targetSlotId, value: 0 });
        writes.push({ record: occupantRecord, field: sourceSlotId, value: 1 });
      }
    } else {
      // Rail → card: displace the occupant, place the dragged player.
      if (occupantRecord !== undefined && occupantRecord !== sourceRecord) {
        writes.push({ record: occupantRecord, field: targetSlotId, value: 0 });
      }
      if (occupantRecord !== sourceRecord) {
        writes.push({ record: sourceRecord, field: targetSlotId, value: 1 });
      }
    }
    void applySlotWrites(writes);
  };

  const bestLines = () => {
    const writes = planBestLines(buckets, entries, summaryOf, slotOwner);
    void applySlotWrites(writes);
  };

  return (
    <div className="space-y-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="flex flex-wrap items-center gap-1 border-b border-slate-800 px-0.5">
          {buckets.map((b) => (
            <BucketTab
              key={b.key}
              label={b.title}
              count={b.units.length}
              active={selectedKey === b.key}
              onClick={() => setActiveBucket(b.key)}
            />
          ))}
          <BucketTab
            label="Scratched"
            count={scratched.length}
            active={selectedKey === SCRATCHED_TAB}
            onClick={() => setActiveBucket(SCRATCHED_TAB)}
          />
        </div>
        <div className="flex items-center gap-2 text-xs text-slate-500">
          <button
            type="button"
            disabled={busy}
            onClick={bestLines}
            className="rounded-md border border-emerald-700/70 bg-emerald-900/30 px-2.5 py-1 text-[11px] font-medium text-emerald-100 hover:bg-emerald-900/50 disabled:cursor-not-allowed disabled:opacity-50"
            title="Fill every unit with the best available players (Overall-ranked by position; scratched players excluded)."
          >
            {busy ? "Working…" : "Best lines"}
          </button>
          <span>
            {totalAssigned}/{totalSlots} slots filled · {entries.length} on roster
          </span>
        </div>
      </div>
      {mutationError != null && <p className="text-xs text-rose-400">{String(mutationError)}</p>}
      <div className="grid gap-3 md:grid-cols-[minmax(0,1fr)_15rem]">
        <div className="min-w-0">
          {selectedKey === SCRATCHED_TAB ? (
            <ScratchedList
              title="Scratched"
              emptyLabel="No scratched players on this team."
              entries={scratched}
              summaryOf={summaryOf}
              busy={statusMut.isPending}
              onRestore={(entry) => statusMut.mutate({ entry, dressed: true })}
            />
          ) : (
            <div className="space-y-4">
              {buckets
                .find((b) => b.key === selectedKey)
                ?.units.map((u) => (
                  <UnitRow
                    key={u.title}
                    unit={u}
                    slotOwner={slotOwner}
                    entryByRecord={entryByRecord}
                    summaryOf={summaryOf}
                    busy={busy}
                    dragOverSlot={dragOverSlot}
                    setDragOverSlot={setDragOverSlot}
                    onDrop={dropToSlot}
                  />
                ))}
            </div>
          )}
        </div>
        <RosterRail
          entries={entries}
          summaryOf={summaryOf}
          activeRecords={new Set(slotOwner.values())}
          busy={busy || statusMut.isPending || archetypeMut.isPending}
          onSetStatus={(entry, dressed) => statusMut.mutate({ entry, dressed })}
          onSetArchetype={(record, value) => archetypeMut.mutate({ record, value })}
        />
      </div>
    </div>
  );
}

function BucketTab({
  label,
  count,
  active,
  onClick,
}: {
  label: string;
  count: number;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={
        "rounded-t-md px-3 py-1.5 text-xs transition-colors " +
        (active
          ? "bg-slate-900/80 text-slate-100 border-b-2 border-sky-500"
          : "text-slate-400 hover:text-slate-200")
      }
    >
      {label}
      <span className="ml-1.5 text-[10px] text-slate-500">({count})</span>
    </button>
  );
}

// ---------------------------------------------------------------------------
// Unit rows + player cards
// ---------------------------------------------------------------------------

function UnitRow({
  unit,
  slotOwner,
  entryByRecord,
  summaryOf,
  busy,
  dragOverSlot,
  setDragOverSlot,
  onDrop,
}: {
  unit: UnitSpec;
  slotOwner: Map<string, number>;
  entryByRecord: Map<number, LineEntryDto>;
  summaryOf: Map<number, PlayerSummaryDto>;
  busy: boolean;
  dragOverSlot: string | null;
  setDragOverSlot: (id: string | null) => void;
  onDrop: (payload: DragPayload, targetSlotId: string) => void;
}) {
  const slots = unit.slots.filter((s) => s.id !== undefined);
  const forwards = slots.slice(0, unit.forwardCount);
  const defense = slots.slice(unit.forwardCount);
  const grid = (cols: SlotSpec[]) => (
    <div
      className="grid gap-1.5"
      style={{ gridTemplateColumns: `repeat(${Math.max(cols.length, 1)}, minmax(0, 1fr))` }}
    >
      {cols.map((s) => (
        <PlayerCard
          key={s.id}
          slot={s}
          occupant={
            slotOwner.has(s.id!) ? entryByRecord.get(slotOwner.get(s.id!)!) : undefined
          }
          summaryOf={summaryOf}
          busy={busy}
          hover={dragOverSlot === s.id}
          setDragOverSlot={setDragOverSlot}
          onDrop={onDrop}
        />
      ))}
    </div>
  );
  return (
    <section>
      <div className="mb-1 flex items-center gap-2">
        <span className="h-px flex-1 bg-slate-800" />
        <span className="px-2 text-[11px] font-semibold uppercase tracking-wider text-slate-300">
          {unit.title}
        </span>
        <span className="h-px flex-1 bg-slate-800" />
      </div>
      {forwards.length > 0 && grid(forwards)}
      {defense.length > 0 && <div className="mt-2">{grid(defense)}</div>}
    </section>
  );
}

function PlayerCard({
  slot,
  occupant,
  summaryOf,
  busy,
  hover,
  setDragOverSlot,
  onDrop,
}: {
  slot: SlotSpec;
  occupant: LineEntryDto | undefined;
  summaryOf: Map<number, PlayerSummaryDto>;
  busy: boolean;
  hover: boolean;
  setDragOverSlot: (id: string | null) => void;
  onDrop: (payload: DragPayload, targetSlotId: string) => void;
}) {
  const slotId = slot.id!;
  const summary = occupant?.player_id != null ? summaryOf.get(occupant.player_id) : undefined;
  const draggable = occupant !== undefined && !busy;
  return (
    <div
      draggable={draggable}
      onDragStart={(e) => {
        if (!draggable || !occupant) return;
        e.dataTransfer.effectAllowed = "move";
        e.dataTransfer.setData(
          LINES_DRAG_MIME,
          JSON.stringify({ slotRecord: occupant.slot_record, sourceSlotId: slotId }),
        );
      }}
      onDragOver={(e) => {
        e.preventDefault();
        e.dataTransfer.dropEffect = "move";
        setDragOverSlot(slotId);
      }}
      onDragLeave={() => setDragOverSlot(null)}
      onDrop={(e) => {
        e.preventDefault();
        setDragOverSlot(null);
        if (busy) return;
        try {
          const payload = JSON.parse(e.dataTransfer.getData(LINES_DRAG_MIME)) as DragPayload;
          if (typeof payload.slotRecord === "number") onDrop(payload, slotId);
        } catch {
          // Not our payload (e.g. a jersey number drag) — ignore.
        }
      }}
      className={
        "overflow-hidden rounded-md border bg-slate-900/60 transition-colors " +
        (hover
          ? "border-sky-500/80 bg-sky-950/40"
          : draggable
            ? "cursor-grab border-slate-800 active:cursor-grabbing"
            : "border-slate-800")
      }
    >
      <div className="border-b border-slate-800 bg-slate-800/40 px-1.5 py-0.5 text-center">
        <span className="text-[10px] font-semibold uppercase tracking-wider text-slate-400">
          {slot.position}
        </span>
      </div>
      <div className="px-1.5 py-1">
        {occupant ? (
          <div className="space-y-0.5 text-center text-[11px]">
            <div className="flex items-baseline justify-center gap-1.5">
              {occupant.captaincy === 1 && (
                <span className="rounded bg-amber-500/20 px-1 font-mono text-[9px] font-bold text-amber-300" title="Captain">
                  C
                </span>
              )}
              {occupant.captaincy === 2 && (
                <span className="rounded bg-sky-500/20 px-1 font-mono text-[9px] font-bold text-sky-300" title="Alternate captain">
                  A
                </span>
              )}
              {occupant.jersey != null && occupant.jersey > 0 && (
                <span className="font-mono text-[10px] tabular-nums text-slate-500">
                  #{occupant.jersey}
                </span>
              )}
              <span className="min-w-0 truncate">
                {occupant.player_id != null ? (
                  <Link
                    to={`/app/database/players/${occupant.player_id}`}
                    className="text-slate-100 hover:text-sky-300"
                  >
                    {occupant.name || `Player #${occupant.player_id}`}
                  </Link>
                ) : (
                  <span className="text-amber-300/90">(unresolved slot)</span>
                )}
              </span>
            </div>
            <div className="flex items-baseline justify-center gap-2 font-mono text-[10px]">
              <span className="text-slate-400" title="Natural position (from player bio)">
                {occupant.position_label || "—"}
              </span>
              <span className="text-slate-500" title={summary?.style_label ?? "Playstyle"}>
                {styleCode(summary?.style_label)}
              </span>
              <span
                className={
                  "text-[11px] font-semibold tabular-nums " + ratingToneClass(summary?.overall)
                }
                title="Overall rating"
              >
                {summary?.overall ?? "—"}
              </span>
            </div>
          </div>
        ) : (
          <div className="text-center text-[11px] italic text-slate-600">vacant</div>
        )}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Scratched / In Minors lists
// ---------------------------------------------------------------------------

function ScratchedList({
  title,
  emptyLabel,
  entries,
  summaryOf,
  busy,
  onRestore,
}: {
  title: string;
  emptyLabel: string;
  entries: LineEntryDto[];
  summaryOf: Map<number, PlayerSummaryDto>;
  busy: boolean;
  onRestore: (entry: LineEntryDto) => void;
}) {
  if (entries.length === 0) {
    return (
      <div className="rounded-md border border-dashed border-slate-800 bg-slate-900/30 px-6 py-8 text-center text-sm text-slate-400">
        {emptyLabel}
      </div>
    );
  }
  const sorted = [...entries].sort((a, b) => a.name.localeCompare(b.name));
  return (
    <div className="rounded-md border border-slate-800 bg-slate-950/40">
      <header className="border-b border-slate-800 px-3 py-1.5 text-xs font-semibold text-slate-200">
        {title} ({sorted.length})
      </header>
      <ul className="divide-y divide-slate-800/60">
        {sorted.map((e) => {
          const summary = e.player_id != null ? summaryOf.get(e.player_id) : undefined;
          return (
            <li key={e.slot_record} className="flex items-baseline gap-2 px-3 py-1.5 text-xs">
              <span className="inline-flex w-7 shrink-0 justify-end font-mono text-[11px] text-slate-500">
                {e.jersey != null && e.jersey > 0 ? `#${e.jersey}` : "—"}
              </span>
              <span className="min-w-0 flex-1 truncate">
                {e.player_id != null ? (
                  <Link
                    to={`/app/database/players/${e.player_id}`}
                    className="text-slate-100 hover:text-sky-300"
                  >
                    {e.name || `Player #${e.player_id}`}
                  </Link>
                ) : (
                  <span className="text-amber-300/90">(unresolved eGlu {e.slot_record})</span>
                )}
              </span>
              {e.position_label && (
                <span className="shrink-0 font-mono text-[10px] text-slate-500">
                  {e.position_label}
                </span>
              )}
              <span
                className={
                  "shrink-0 font-mono text-[11px] font-semibold tabular-nums " +
                  ratingToneClass(summary?.overall)
                }
              >
                {summary?.overall ?? "—"}
              </span>
              <button
                type="button"
                onClick={() => onRestore(e)}
                disabled={busy || e.player_id == null}
                title="Restore to the active roster (Dressed)"
                className="rounded border border-slate-700 px-1.5 py-0.5 text-[10px] text-slate-300 hover:bg-slate-800 disabled:opacity-30"
              >
                Dress
              </button>
            </li>
          );
        })}
      </ul>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Roster rail — draggable player list with status + archetype controls
// ---------------------------------------------------------------------------

function RosterRail({
  entries,
  summaryOf,
  activeRecords,
  busy,
  onSetStatus,
  onSetArchetype,
}: {
  entries: LineEntryDto[];
  summaryOf: Map<number, PlayerSummaryDto>;
  activeRecords: Set<number>;
  busy: boolean;
  onSetStatus: (entry: LineEntryDto, dressed: boolean) => void;
  onSetArchetype: (record: number, value: number) => void;
}) {
  const [filter, setFilter] = useState<"All" | "C" | "LW" | "RW" | "D" | "G">("All");
  const byOverallThenName = (a: LineEntryDto, b: LineEntryDto) => {
    const ao = (a.player_id != null && summaryOf.get(a.player_id)?.overall) || -1;
    const bo = (b.player_id != null && summaryOf.get(b.player_id)?.overall) || -1;
    if (ao !== bo) return bo - ao;
    return a.name.localeCompare(b.name);
  };
  const isGoalie = (e: LineEntryDto) => e.position_label === "G";
  const matches = (e: LineEntryDto) =>
    filter === "All" || (filter === "G" ? isGoalie(e) : e.position_label === filter);
  const skaters = entries.filter((e) => !isGoalie(e) && matches(e)).sort(byOverallThenName);
  const goalies = entries.filter((e) => isGoalie(e) && matches(e)).sort(byOverallThenName);

  return (
    <aside className="rounded-md border border-slate-800 bg-slate-950/40">
      <header className="space-y-1.5 border-b border-slate-800 px-3 py-2">
        <div className="text-xs font-semibold text-slate-200">
          Roster
          <span className="ml-2 text-[10px] font-normal text-slate-500">
            ({skaters.length + goalies.length})
          </span>
        </div>
        <select
          value={filter}
          onChange={(e) => setFilter(e.target.value as typeof filter)}
          aria-label="Filter roster by position"
          className="w-full rounded border border-slate-700 bg-slate-950 px-2 py-1 text-[11px] text-slate-200"
        >
          <option value="All">All positions</option>
          <option value="C">Centers</option>
          <option value="LW">Left wings</option>
          <option value="RW">Right wings</option>
          <option value="D">Defensemen</option>
          <option value="G">Goalies</option>
        </select>
      </header>
      <div className="max-h-[60vh] overflow-auto">
        {skaters.length > 0 && (
          <ul className="divide-y divide-slate-800/60">
            {skaters.map((e) => (
              <RailRow
                key={e.slot_record}
                entry={e}
                summary={e.player_id != null ? summaryOf.get(e.player_id) : undefined}
                goalie={false}
                onLine={activeRecords.has(e.slot_record)}
                busy={busy}
                onSetStatus={onSetStatus}
                onSetArchetype={onSetArchetype}
              />
            ))}
          </ul>
        )}
        {goalies.length > 0 && (
          <>
            <div className="flex items-center gap-2 border-y border-slate-800 bg-slate-900/40 px-3 py-1">
              <span className="text-[10px] font-semibold uppercase tracking-wider text-slate-400">
                Goalies
              </span>
              <span className="ml-auto font-mono text-[10px] text-slate-500">{goalies.length}</span>
            </div>
            <ul className="divide-y divide-slate-800/60">
              {goalies.map((e) => (
                <RailRow
                  key={e.slot_record}
                  entry={e}
                  summary={e.player_id != null ? summaryOf.get(e.player_id) : undefined}
                  goalie
                  onLine={activeRecords.has(e.slot_record)}
                  busy={busy}
                  onSetStatus={onSetStatus}
                  onSetArchetype={onSetArchetype}
                />
              ))}
            </ul>
          </>
        )}
      </div>
      <footer className="border-t border-slate-800 px-3 py-1.5 text-[10px] text-slate-500">
        Drag a player onto a line slot to place them.
      </footer>
    </aside>
  );
}

/** Rail options for the roster style cell: the domain's per-title labels for skaters, else the
 *  field's declared range as plain numbers (goalies always numbers). */
export function archetypeChoices(style: FieldViewDto | null, goalie: boolean): Array<{ raw: number; label: string }> {
  if (!style) return [];
  if (!goalie && style.enum_options.length > 0) return style.enum_options.map((o) => ({ raw: o.raw, label: o.label })).sort((a, b) => a.raw - b.raw);
  const lo = style.range_low ?? 0;
  const hi = style.range_high ?? lo;
  const out: Array<{ raw: number; label: string }> = [];
  for (let r = lo; r <= hi && out.length < 64; r++) out.push({ raw: r, label: String(r) });
  return out;
}

function RailRow({
  entry,
  summary,
  goalie,
  onLine,
  busy,
  onSetStatus,
  onSetArchetype,
}: {
  entry: LineEntryDto;
  summary: PlayerSummaryDto | undefined;
  /** Goalie rows: the roster cell's meaning for goalies is unresolved (research R122 S4) — numbers only. */
  goalie: boolean;
  onLine: boolean;
  busy: boolean;
  onSetStatus: (entry: LineEntryDto, dressed: boolean) => void;
  onSetArchetype: (record: number, value: number) => void;
}) {
  const outOfLineup = !isDressed(entry.dressed);
  const unassigned = !outOfLineup && !onLine;
  const inactive = summary?.is_active === false;
  const archetypeRaw = entry.style?.decoded ?? null;
  // The per-title vocabulary the domain attached to this cell (research R122: NHL 11+ the Legacy
  // table, NHL 08 / 09 the NHL 07 set, NHL 07 / 10 none); without one, the field's own range as
  // numbers — never a borrowed label, and never a code the field cannot hold.
  const archetypeOptions = archetypeChoices(entry.style ?? null, goalie);
  if (archetypeRaw !== null && !archetypeOptions.some((o) => o.raw === archetypeRaw)) {
    archetypeOptions.push({ raw: archetypeRaw, label: String(archetypeRaw) });
  }

  return (
    <li
      className={
        "px-3 py-1.5 text-xs hover:bg-slate-800/60 " +
        (outOfLineup
          ? "bg-slate-900/50 text-slate-500"
          : unassigned
            ? "bg-amber-950/20 text-amber-200"
            : inactive
              ? "text-slate-500"
              : "text-slate-100")
      }
    >
      <div
        draggable={!outOfLineup && !busy}
        onDragStart={(e) => {
          if (outOfLineup) return;
          e.dataTransfer.effectAllowed = "move";
          e.dataTransfer.setData(
            LINES_DRAG_MIME,
            JSON.stringify({ slotRecord: entry.slot_record, sourceSlotId: null }),
          );
        }}
        className={
          "flex items-baseline gap-2 " +
          (outOfLineup ? "cursor-default" : "cursor-grab active:cursor-grabbing")
        }
        title={
          outOfLineup
            ? `${entry.name} is out of the lineup`
            : unassigned
              ? "Dressed but not assigned to any line"
              : "Drag to a slot"
        }
      >
        <span className="inline-flex w-6 shrink-0 justify-end font-mono text-[10px] text-slate-500">
          {entry.jersey != null && entry.jersey > 0 ? `#${entry.jersey}` : "—"}
        </span>
        <span className="min-w-0 flex-1 truncate">{entry.name || `#${entry.player_id}`}</span>
        {entry.captaincy === 1 && (
          <span className="shrink-0 rounded bg-amber-500/20 px-1 font-mono text-[9px] font-bold text-amber-300" title="Captain">
            C
          </span>
        )}
        {entry.captaincy === 2 && (
          <span className="shrink-0 rounded bg-sky-500/20 px-1 font-mono text-[9px] font-bold text-sky-300" title="Alternate captain">
            A
          </span>
        )}
        {entry.position_label && (
          <span className="shrink-0 font-mono text-[10px] text-slate-500">
            {entry.position_label}
          </span>
        )}
        <span
          className={
            "shrink-0 font-mono text-[11px] font-semibold tabular-nums " +
            ratingToneClass(summary?.overall)
          }
          title="Overall rating"
        >
          {summary?.overall ?? "—"}
        </span>
      </div>
      <div className="mt-1 flex items-center gap-1">
        <select
          value={outOfLineup ? "scratched" : "dressed"}
          disabled={busy || entry.player_id == null}
          onChange={(e) => onSetStatus(entry, e.target.value === "dressed")}
          aria-label="Lineup status"
          title="Lineup status (eGlu.hSZj: dressed or scratched)"
          className="min-w-0 flex-1 rounded border border-slate-700 bg-slate-950 px-1 py-0.5 text-[10px] text-slate-300 disabled:opacity-30"
        >
          <option value="dressed">Dressed</option>
          <option value="scratched">Scratched</option>
        </select>
        <select
          value={archetypeRaw ?? ""}
          disabled={busy || archetypeRaw === null || entry.style?.editable === false}
          onChange={(e) => onSetArchetype(entry.slot_record, Number(e.target.value))}
          aria-label="Archetype"
          title={entry.style?.vocabulary_note ?? (goalie ? "Per-slot style code (eGlu.QgFs): its meaning for goalies is unresolved — shown as a number" : "Per-slot style (eGlu.QgFs)")}
          className="min-w-0 flex-1 rounded border border-amber-900/50 bg-slate-950 px-1 py-0.5 text-[10px] text-amber-200/80 disabled:opacity-30"
        >
          {archetypeRaw === null && <option value="">—</option>}
          {archetypeOptions.map((o) => (
            <option key={o.raw} value={o.raw}>
              {o.label}
            </option>
          ))}
        </select>
      </div>
    </li>
  );
}

// ---------------------------------------------------------------------------
// Best lines — Overall-ranked auto-fill (the reference algorithm)
// ---------------------------------------------------------------------------

function planBestLines(
  buckets: BucketSpec[],
  entries: LineEntryDto[],
  summaryOf: Map<number, PlayerSummaryDto>,
  slotOwner: Map<string, number>,
): Array<{ record: number; field: string; value: 0 | 1 }> {
  const candidates = entries
    .filter((e) => e.dressed !== false && e.player_id != null)
    .map((e) => ({ entry: e, summary: summaryOf.get(e.player_id as number) }))
    .filter((c): c is { entry: LineEntryDto; summary: PlayerSummaryDto } => c.summary !== undefined)
    .sort((a, b) => {
      const d = (b.summary.overall ?? -1) - (a.summary.overall ?? -1);
      if (d !== 0) return d;
      return a.entry.name.localeCompare(b.entry.name);
    });
  const pos = (p: string) => candidates.filter((c) => c.entry.position_label === p);
  const goalies = pos("G");
  const skaters = candidates.filter((c) => c.entry.position_label !== "G");
  const forwards = skaters.filter((c) => c.entry.position_label !== "D");
  const defense = pos("D");

  // Desired occupant per slot id.
  const desired = new Map<string, number>();
  const take = (slots: SlotSpec[], ranked: typeof candidates, offset = 0) => {
    slots.forEach((s, i) => {
      const c = ranked[offset + i];
      if (s.id && c) desired.set(s.id, c.entry.slot_record);
    });
  };

  for (const bucket of buckets) {
    if (bucket.key === "even") {
      const lines = bucket.units.filter((u) => u.title.startsWith("Offense"));
      take(lines.map((u) => u.slots[0]), pos("LW"));
      take(lines.map((u) => u.slots[1]), pos("C"));
      take(lines.map((u) => u.slots[2]), pos("RW"));
      const pairs = bucket.units.filter((u) => u.title.startsWith("Defense"));
      // Interleave LD1 RD1 LD2 RD2 … so the top pair gets the two best defensemen.
      const dSlots: SlotSpec[] = [];
      for (const p of pairs) dSlots.push(...p.slots);
      take(dSlots, defense);
      const g = bucket.units.find((u) => u.title === "Goalies");
      if (g) take(g.slots, goalies);
    } else if (bucket.key === "so" || bucket.key === "x") {
      for (const u of bucket.units) take(u.slots, skaters);
    } else {
      // Special teams / OT: unit N takes the next forwardCount forwards + defenseCount defense.
      const groups = new Map<string, UnitSpec[]>();
      for (const u of bucket.units) {
        const prefix = u.title.replace(/· Unit \d+$/, "");
        groups.set(prefix, [...(groups.get(prefix) ?? []), u]);
      }
      for (const units of groups.values()) {
        units.forEach((u, unitIndex) => {
          const fwd = u.slots.slice(0, u.forwardCount);
          const def = u.slots.slice(u.forwardCount);
          take(fwd, forwards, unitIndex * u.forwardCount);
          take(def, defense, unitIndex * def.length);
        });
      }
    }
  }

  // Final bit-writes: clear the wrong occupant, set the desired one — skipping already-right slots.
  const writes = new Map<string, { record: number; field: string; value: 0 | 1 }>();
  for (const [slotId, record] of desired) {
    const current = slotOwner.get(slotId);
    if (current === record) continue;
    if (current !== undefined) {
      writes.set(`${current}|${slotId}`, { record: current, field: slotId, value: 0 });
    }
    writes.set(`${record}|${slotId}`, { record, field: slotId, value: 1 });
  }
  return [...writes.values()];
}

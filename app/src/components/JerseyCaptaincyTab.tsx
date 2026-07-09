import { useState } from "react";
import { Link } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { EmptyState, Spinner } from "./primitives";
import { isDressedStatus } from "../lib/teamPresentation";
import type { LineEntryDto } from "../bindings/LineEntryDto";

// HTML5 drag-and-drop MIME. Custom MIME types aren't reliably carried through Tauri's webview
// (`dataTransfer.types` can omit them on dragover, suppressing the drop) — `text/plain` is the
// universal lowest-common-denominator; drop handlers validate the payload (a number in 1..99).
const DRAG_MIME = "text/plain";
const NUMBERS = Array.from({ length: 99 }, (_, i) => i + 1);
const GRID_COLS = 9;

/// The Jersey Numbers & Captaincy tab — the reference layout: a draggable roster list on the
/// left (drop a number onto a player's slot to assign it), the 99-number grid with per-number
/// retired checkboxes in the middle, and the Captain / Alternate pickers on the right.
///
///  - Jersey assignments write the per-team slot number (`eGlu.sVRt`) through the audited
///    `set_jersey_number` op (unique within the team; dropping a taken number steals it —
///    the previous holder's number is cleared first).
///  - Retired-number toggles append / remove `wcSL` rows (`retire_jersey` / `unretire_jersey`).
///  - Captaincy picks go through `set_captaincy` (`eGlu.mCcl`), which auto-demotes a prior
///    captain; only DRESSED players (`ZBac.RREB` not Scratched / In-Minors) may hold a role.
export function JerseyCaptaincyTab({
  projectId,
  teamId,
  retiredNumbers,
}: {
  projectId: number;
  teamId: number;
  /** The team's retired set from `TeamViewDto.retired_numbers` (`wcSL`). */
  retiredNumbers: number[];
}) {
  const qc = useQueryClient();
  const { update } = useProject();
  const lines = useQuery({
    queryKey: ["lines", projectId, teamId],
    queryFn: () => call("lines_view", { projectId, teamId }),
  });

  const refresh = async () => {
    await qc.invalidateQueries();
    update(await call("project_summary", { projectId }));
  };

  const assignMut = useMutation({
    // Steal-aware assign: clear the current holder's number first, then set the new one.
    mutationFn: async (v: { player: number; number: number; stealFrom?: number }) => {
      if (v.stealFrom !== undefined) {
        await call("set_jersey_number", { projectId, team: teamId, player: v.stealFrom, number: 0 });
      }
      return call("set_jersey_number", { projectId, team: teamId, player: v.player, number: v.number });
    },
    onSuccess: refresh,
  });
  const retiredMut = useMutation({
    mutationFn: (v: { number: number; retire: boolean }) =>
      call(v.retire ? "retire_jersey" : "unretire_jersey", {
        projectId,
        team: teamId,
        number: v.number,
      }),
    onSuccess: refresh,
  });
  const captaincyMut = useMutation({
    mutationFn: async (v: { demote?: number; promote?: number; role: number }) => {
      if (v.demote !== undefined) {
        await call("set_captaincy", { projectId, team: teamId, player: v.demote, role: 0 });
      }
      if (v.promote !== undefined) {
        return call("set_captaincy", { projectId, team: teamId, player: v.promote, role: v.role });
      }
      return null;
    },
    onSuccess: refresh,
  });

  if (lines.isLoading) return <Spinner label="Loading roster…" />;
  if (lines.isError) return <p className="text-rose-400">{String(lines.error)}</p>;
  const entries = (lines.data?.entries ?? []).filter((e) => e.player_id != null);
  if (entries.length === 0) {
    return <EmptyState title="No players are listed under this team's roster" />;
  }

  const sorted = [...entries].sort((a, b) => a.name.localeCompare(b.name));
  const holderOf = (n: number) => sorted.find((e) => e.jersey === n);
  const assigned = new Set(sorted.map((e) => e.jersey).filter((n): n is number => n != null && n > 0));
  const retired = new Set(retiredNumbers);
  const dressed = new Set(
    sorted.filter((e) => isDressedStatus(e.roster_status)).map((e) => e.player_id as number),
  );
  const captain = sorted.find((e) => e.captaincy === 1)?.player_id ?? null;
  const alternates = sorted.filter((e) => e.captaincy === 2).map((e) => e.player_id as number);

  const busy = assignMut.isPending || retiredMut.isPending || captaincyMut.isPending;
  const error = assignMut.error ?? retiredMut.error ?? captaincyMut.error;

  const assign = (player: number, number: number) => {
    const holder = holderOf(number);
    if (holder?.player_id === player) return;
    assignMut.mutate({
      player,
      number,
      stealFrom: holder?.player_id != null ? holder.player_id : undefined,
    });
  };

  return (
    <div className="space-y-3">
      {error != null && <p className="text-xs text-rose-400">{String(error)}</p>}
      <div className="flex flex-wrap items-start gap-4">
        <RosterList
          entries={sorted}
          busy={busy}
          onAssign={assign}
          onClear={(player) => assignMut.mutate({ player, number: 0 })}
        />
        <div className="flex flex-col gap-4">
          <NumberGrid
            assigned={assigned}
            retired={retired}
            busy={retiredMut.isPending}
            onToggleRetired={(n) => retiredMut.mutate({ number: n, retire: !retired.has(n) })}
          />
          <CaptaincyPanel
            entries={sorted}
            dressed={dressed}
            captain={captain}
            alternates={alternates}
            busy={captaincyMut.isPending}
            onChange={(slotValue, oldPid, newPid, role) => {
              void slotValue;
              captaincyMut.mutate({
                demote: oldPid ?? undefined,
                promote: newPid ?? undefined,
                role,
              });
            }}
          />
        </div>
      </div>
      <p className="text-[11px] text-slate-500">
        Drag a number onto a player's slot to assign it; numbers steal from whoever currently
        holds them. Assignments write the per-team slot number (eGlu.sVRt), retired checkboxes
        write wcSL rows, and captaincy picks write eGlu.mCcl — all audited and undoable.
      </p>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Left: roster list with jersey drop slots
// ---------------------------------------------------------------------------

function RosterList({
  entries,
  busy,
  onAssign,
  onClear,
}: {
  entries: LineEntryDto[];
  busy: boolean;
  onAssign: (player: number, number: number) => void;
  onClear: (player: number) => void;
}) {
  return (
    <section className="flex w-72 shrink-0 flex-col rounded-lg border border-slate-800 bg-slate-900/40">
      <header className="flex items-baseline justify-between border-b border-slate-800 px-3 py-2">
        <span className="text-[11px] font-semibold uppercase tracking-wide text-slate-200">
          Roster
        </span>
        <span className="text-[11px] text-slate-500">{entries.length} players</span>
      </header>
      <ul className="max-h-[70vh] divide-y divide-slate-800/60 overflow-y-auto">
        {entries.map((e) => (
          <RosterRow
            key={e.slot_record}
            entry={e}
            busy={busy}
            onAssign={(n) => onAssign(e.player_id as number, n)}
            onClear={() => onClear(e.player_id as number)}
          />
        ))}
      </ul>
    </section>
  );
}

function RosterRow({
  entry,
  busy,
  onAssign,
  onClear,
}: {
  entry: LineEntryDto;
  busy: boolean;
  onAssign: (number: number) => void;
  onClear: () => void;
}) {
  const [dragOver, setDragOver] = useState(false);
  const jersey = entry.jersey != null && entry.jersey > 0 ? entry.jersey : null;
  const filled = jersey !== null;

  return (
    <li className="flex items-center gap-2 px-3 py-1.5 text-xs">
      <div
        onDragOver={(e) => {
          // Always accept; the payload is validated in onDrop (Tauri's webview drops custom
          // MIME entries from dataTransfer.types during dragover).
          e.preventDefault();
          e.dataTransfer.dropEffect = "move";
          setDragOver(true);
        }}
        onDragLeave={() => setDragOver(false)}
        onDrop={(e) => {
          e.preventDefault();
          setDragOver(false);
          if (busy) return;
          const n = Number(e.dataTransfer.getData(DRAG_MIME));
          if (Number.isFinite(n) && n >= 1 && n <= 99) onAssign(n);
        }}
        className={
          "group relative flex h-8 w-10 shrink-0 items-center justify-center rounded-md border font-mono text-xs transition-colors " +
          (dragOver
            ? "border-sky-400 bg-sky-900/30 text-sky-100"
            : filled
              ? "border-slate-700 bg-slate-800/60 text-slate-100"
              : "border-dashed border-slate-700 bg-slate-950/40 text-slate-600")
        }
        title={filled ? "Drag to reassign · click × to clear" : "Drop a number here"}
      >
        {filled ? (
          <>
            <span
              draggable
              onDragStart={(e) => {
                e.dataTransfer.setData(DRAG_MIME, String(jersey));
                e.dataTransfer.effectAllowed = "move";
              }}
              className="cursor-grab select-none active:cursor-grabbing"
            >
              #{jersey}
            </span>
            <button
              type="button"
              onClick={onClear}
              disabled={busy}
              aria-label="Clear jersey number"
              className="absolute -right-1.5 -top-1.5 hidden h-4 w-4 items-center justify-center rounded-full border border-slate-700 bg-slate-900 text-[9px] text-slate-400 hover:text-rose-300 group-hover:flex"
            >
              ×
            </button>
          </>
        ) : (
          <span className="text-slate-600">—</span>
        )}
      </div>
      <Link
        to={`/app/database/players/${entry.player_id}`}
        className="min-w-0 flex-1 truncate text-slate-100 hover:text-sky-300"
      >
        {entry.name || `Player #${entry.player_id}`}
      </Link>
      {entry.position_label && (
        <span className="shrink-0 font-mono text-[10px] text-slate-500">
          {entry.position_label}
        </span>
      )}
    </li>
  );
}

// ---------------------------------------------------------------------------
// Middle: the 99-number grid with retired toggles
// ---------------------------------------------------------------------------

function NumberGrid({
  assigned,
  retired,
  busy,
  onToggleRetired,
}: {
  assigned: Set<number>;
  retired: Set<number>;
  busy: boolean;
  onToggleRetired: (n: number) => void;
}) {
  return (
    <section className="rounded-lg border border-slate-800 bg-slate-900/40">
      <header className="border-b border-slate-800 px-3 py-2">
        <div className="flex items-baseline justify-between">
          <span className="text-[11px] font-semibold uppercase tracking-wide text-slate-200">
            Numbers
          </span>
          <span className="text-[11px] text-slate-500">
            {assigned.size} assigned · {retired.size} retired
          </span>
        </div>
        <p className="mt-0.5 text-[10px] text-slate-500">
          Drag a number onto a player's slot to assign it. The checkbox beside each number marks
          it as a retired jersey.
        </p>
      </header>
      <div className="grid w-fit gap-1.5 p-3" style={{ gridTemplateColumns: `repeat(${GRID_COLS}, auto)` }}>
        {NUMBERS.map((n) => (
          <div key={n} className="flex items-center gap-1">
            <span
              draggable
              onDragStart={(e) => {
                e.dataTransfer.setData(DRAG_MIME, String(n));
                e.dataTransfer.effectAllowed = "move";
              }}
              className={
                "flex h-7 w-8 cursor-grab select-none items-center justify-center rounded-md border font-mono text-xs tabular-nums transition-colors active:cursor-grabbing " +
                (retired.has(n)
                  ? "border-orange-700/70 bg-orange-900/40 text-orange-300"
                  : "border-slate-800 bg-slate-950/40 text-slate-100 hover:border-slate-700") +
                (assigned.has(n) ? " opacity-40" : "")
              }
              title={retired.has(n) ? `#${n} — retired` : assigned.has(n) ? `#${n} — assigned` : `#${n}`}
            >
              {n}
            </span>
            <input
              type="checkbox"
              checked={retired.has(n)}
              disabled={busy}
              onChange={() => onToggleRetired(n)}
              aria-label={`Mark #${n} as retired`}
              title="Retired"
              className="h-3 w-3 cursor-pointer accent-orange-500"
            />
          </div>
        ))}
      </div>
    </section>
  );
}

// ---------------------------------------------------------------------------
// Right: Captain + two Alternates
// ---------------------------------------------------------------------------

function CaptaincyPanel({
  entries,
  dressed,
  captain,
  alternates,
  busy,
  onChange,
}: {
  entries: LineEntryDto[];
  dressed: Set<number>;
  captain: number | null;
  alternates: number[];
  busy: boolean;
  onChange: (slot: string, oldPid: number | null, newPid: number | null, role: number) => void;
}) {
  const alt1 = alternates[0] ?? null;
  const alt2 = alternates[1] ?? null;
  const picker = (
    label: string,
    badge: string,
    value: number | null,
    excluded: Set<number>,
    role: number,
    slot: string,
  ) => (
    <label className="flex flex-col gap-1 text-xs">
      <span className="flex items-center gap-2 text-[10px] uppercase tracking-wide text-slate-500">
        {label}
        <span
          className={
            "rounded px-1 font-mono text-[9px] font-bold " +
            (badge === "C" ? "bg-amber-500/20 text-amber-300" : "bg-sky-500/20 text-sky-300")
          }
        >
          {badge}
        </span>
      </span>
      <select
        value={value ?? ""}
        disabled={busy}
        onChange={(e) => {
          const v = e.target.value === "" ? null : Number(e.target.value);
          if (v === value) return;
          onChange(slot, value, v, role);
        }}
        className="rounded border border-slate-700 bg-slate-950 px-1.5 py-0.5 font-mono text-xs text-slate-100 focus:border-sky-600 focus:outline-none"
      >
        <option value="">— none —</option>
        {entries.map((p) => {
          const pid = p.player_id as number;
          const notDressed = !dressed.has(pid);
          const disabled = pid !== value && (excluded.has(pid) || notDressed);
          return (
            <option key={pid} value={pid} disabled={disabled}>
              {p.name || `Player #${pid}`}
              {p.jersey != null && p.jersey > 0 ? `  #${p.jersey}` : ""}
              {notDressed ? "  (not dressed)" : ""}
            </option>
          );
        })}
      </select>
    </label>
  );

  const not = (vals: Array<number | null>) =>
    new Set(vals.filter((v): v is number => v !== null));

  return (
    <section className="w-72 shrink-0 rounded-lg border border-slate-800 bg-slate-900/40">
      <header className="border-b border-slate-800 px-3 py-2 text-[11px] font-semibold uppercase tracking-wide text-slate-200">
        Captaincy
      </header>
      <div className="space-y-3 px-3 py-3">
        {picker("Captain", "C", captain, not([alt1, alt2]), 1, "captain")}
        {picker("Alternate 1", "A", alt1, not([captain, alt2]), 2, "alt1")}
        {picker("Alternate 2", "A", alt2, not([captain, alt1]), 2, "alt2")}
      </div>
      <p className="border-t border-slate-800 px-3 py-2 text-[10px] text-slate-500">
        Only dressed players can be assigned a captaincy role. Scratched / in-minors players are
        disabled — set them to Dressed on the Lines tab first.
      </p>
    </section>
  );
}

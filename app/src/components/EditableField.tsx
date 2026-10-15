import { useEffect, useRef, useState, type ReactNode } from "react";
import { Link } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { EnumSelect } from "./EnumSelect";
import { useFieldEdit } from "../lib/useFieldEdit";
import { clampRaw, friendlySeed, friendlyToRaw, hasPlainBounds } from "../lib/editForm";
import type { FieldViewDto } from "../bindings/FieldViewDto";
import type { TeamViewDto } from "../bindings/TeamViewDto";
import type { ArenaAssignmentPreviewDto } from "../bindings/ArenaAssignmentPreviewDto";

/// Click-to-edit shell (the reference app's pattern): the resolved value renders as plain text
/// until clicked, then the real control mounts focused. Closes on commit (Enter, a dropdown pick,
/// or blur — the control's own blur/Enter handlers stage the edit first as the event bubbles) or
/// on Escape (the control unmounts without a blur, so nothing commits). Keeps dense field grids
/// calm instead of a wall of input boxes.
export function ClickToEdit({
  display,
  tone,
  children,
}: {
  /** The closed-state text (the field's resolved display value). */
  display: ReactNode;
  /** Optional text-colour class for the closed state (e.g. rating tier colour). */
  tone?: string;
  children: ReactNode;
}) {
  const [editing, setEditing] = useState(false);
  const boxRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!editing) return;
    const el = boxRef.current?.querySelector<HTMLElement>("input, select");
    el?.focus();
    if (el instanceof HTMLInputElement) el.select();
  }, [editing]);

  if (!editing) {
    const empty = display == null || display === "";
    return (
      <button
        type="button"
        title="Click to edit"
        onClick={() => setEditing(true)}
        className={`block w-full cursor-text truncate rounded px-0.5 py-0.5 text-left font-mono text-xs hover:bg-slate-800/60 ${
          tone ? `${tone} font-semibold` : "text-slate-100"
        }`}
      >
        {empty ? <span className="text-slate-600">—</span> : display}
      </button>
    );
  }
  return (
    <div
      ref={boxRef}
      onKeyDown={(e) => {
        if (e.key === "Escape" || e.key === "Enter") setEditing(false);
      }}
      onChange={(e) => {
        // A dropdown pick is the commit — close like the reference app. Text inputs fire change
        // per keystroke; they close on Enter/blur instead.
        if ((e.target as HTMLElement).tagName === "SELECT") setEditing(false);
      }}
      onBlur={(e) => {
        if (!e.currentTarget.contains(e.relatedTarget as Node)) setEditing(false);
      }}
    >
      {children}
    </div>
  );
}

/// A stacked (label-over-control) inline field editor for the sectioned Player Data layout. Picks the
/// control from the field's shape: an enum-bound field → a dropdown of its vocabulary; a 1-bit field
/// → a checkbox; a string → a text box; anything else → a number box. Non-updatable fields render
/// their resolved display read-only. All edits stage through `set_field` and refresh the open views.
export function EditableField({
  projectId,
  record,
  field: sourceField,
  label,
  tone,
}: {
  projectId: number;
  record: number;
  field: FieldViewDto;
  label: string;
  /** Optional text-colour class for the value (e.g. rating tier colour). Applies to numeric/read-only
   *  displays; enum dropdowns and checkboxes ignore it. */
  tone?: string;
}) {
  const [refusal, setRefusal] = useState<string | null>(null);
  const edit = useFieldEdit(projectId, record, sourceField);
  const field = edit.field!;
  const mut = { ...edit, mutate: (value: { intValue?: number; textValue?: string }) => {
    setRefusal(null);
    edit.mutate(value);
  } };
  const isFlag =
    !field.is_string && field.range_low === 0 && field.range_high === 1 && field.enum_options.length === 0;

  let control: ReactNode;
  if (!field.editable) {
    control = <span className={`text-xs ${tone ?? "text-slate-100"}`}>{field.display || "—"}</span>;
  } else if (field.enum_options.length > 0) {
    control = (
      <ClickToEdit display={field.display}>
          <EnumSelect
            label={label}
            value={field.decoded ?? 0}
            options={field.enum_options}
            onChange={(raw) => mut.mutate({ intValue: raw })}
          />
      </ClickToEdit>
    );
  } else if (isFlag) {
    // A checkbox is already its own click-to-edit — it stays visible.
    control = (
      <Checkbox checked={field.decoded === 1} onChange={(on) => mut.mutate({ intValue: on ? 1 : 0 })} />
    );
  } else {
    control = (
      <ClickToEdit display={field.display} tone={tone}>
        <TextNumber field={field} tone={tone} onCommit={(v) => mut.mutate(v)} onRefuse={setRefusal} />
      </ClickToEdit>
    );
  }

  return (
    <Labeled label={label} pending={mut.isPending} error={mut.error}>
      {control}
      {refusal && (
        <span className="mt-0.5 block text-[11px] text-rose-300" data-testid="text-too-long">
          {refusal}
        </span>
      )}
    </Labeled>
  );
}

type TeamOption = { team_id: number; name: string; abbrev: string };

/// A team dropdown that edits a team-FK bio field. Most refs (`dbBW`/`ZbNy`/`vgWu`) are 1-indexed —
/// stored value is `team_id + 1`, `0` = "none / free agent". The "main team" `dXSB` drifted: it is
/// 0-indexed on NHL 09/10 (stored value **is** the team id, no free-agent sentinel). Pass `offset`
/// (0 or 1, default 1) — the caller reads it from `capabilities.main_team_offset` for `dXSB`.
/// Read-only if the field is absent or not updatable.
export function TeamSelect({
  projectId,
  record,
  field: sourceField,
  teams,
  label,
  noneLabel = "(none — Free Agent)",
  offset = 1,
}: {
  projectId: number;
  record: number;
  field: FieldViewDto | undefined;
  teams: TeamOption[];
  label: string;
  noneLabel?: string;
  offset?: number;
}) {
  const edit = useFieldEdit(projectId, record, sourceField);
  const field = edit.field;
  const mut = { ...edit, mutate: (raw: number) => edit.mutate({ intValue: raw }) };
  const raw = field?.decoded ?? 0;
  // 1-indexed refs reserve raw 0 for "free agent"; 0-indexed `dXSB` (NHL 09/10) has no such
  // sentinel — every raw value is a real team id.
  const teamId = offset === 1 ? (raw > 0 ? raw - 1 : -1) : raw;
  const canBeNone = offset === 1;

  if (!field || !field.editable) {
    const t = teams.find((x) => x.team_id === teamId);
    return (
      <ReadonlyField label={label}>{teamId >= 0 ? t?.name ?? `Team ${teamId}` : ""}</ReadonlyField>
    );
  }

  const sorted = [...teams].sort((a, b) => a.name.localeCompare(b.name));
  const currentName =
    teamId >= 0 ? teams.find((t) => t.team_id === teamId)?.name ?? `Team ${teamId}` : noneLabel;
  return (
    <Labeled label={label} pending={mut.isPending} error={mut.error}>
      <ClickToEdit display={currentName}>
        <select
          className={CONTROL}
          value={teamId}
          onChange={(e) => mut.mutate(Number(e.target.value) + offset)}
        >
          {canBeNone && <option value={-1}>{noneLabel}</option>}
          {sorted.map((t) => (
            <option key={t.team_id} value={t.team_id}>
              {t.name}
              {t.abbrev ? ` (${t.abbrev})` : ""}
            </option>
          ))}
        </select>
      </ClickToEdit>
    </Labeled>
  );
}

/// The home-arena picker: `kOtt.buDa` stores the 0-indexed `StEO` id directly (no "none"
/// sentinel), so options map 1:1 onto the arena directory. The arena name is a link to its entry in
/// the Arenas tab; editing goes through the small "change" affordance beside it (the text itself
/// can't be click-to-edit and a navigation link at the same time).
/** The team's home-arena cell (Teams 2.2, 24 Sep 2026): a **disclosed two-step assignment**.
 *  Picking an arena writes nothing; the backend's `arena_assignment_preview` is shown first — the
 *  exact cell, who already plays at the chosen arena, who stays at the current one or that it
 *  becomes vacant, or the refusal (same arena, uncatalogued id, no link on this title) — and only
 *  **Assign** writes, through the ordinary validated `set_field` on the previewed cell with the
 *  previewed arena id. The candidate and its preview are bound to (project, team, record, field);
 *  a change of any of those drops them, and a completion landing after such a change is ignored. */
export function ArenaSelect({
  projectId,
  teamId,
  record,
  field,
  arena,
}: {
  projectId: number;
  teamId: number;
  record: number;
  field: FieldViewDto;
  arena: TeamViewDto["arena"];
}) {
  const qc = useQueryClient();
  const { update } = useProject();
  const [editing, setEditing] = useState(false);
  const [candidate, setCandidate] = useState<number | null>(null);
  const targetKey = `${projectId}/${teamId}/${field.studio_table}/${record}/${field.studio_field}`;
  const targetRef = useRef(targetKey);
  const genRef = useRef(0);
  useEffect(() => {
    if (targetRef.current === targetKey) return;
    targetRef.current = targetKey;
    genRef.current += 1;
    setCandidate(null);
    setEditing(false);
  }, [targetKey]);

  const arenasQ = useQuery({
    queryKey: ["arenas", projectId],
    queryFn: () => call("list_arenas", { projectId }),
  });
  const arenas = arenasQ.data ?? [];
  const preview = useQuery({
    queryKey: ["arenaAssignmentPreview", projectId, teamId, record, field.studio_field, candidate],
    queryFn: () => call("arena_assignment_preview", { projectId, teamId, toArena: candidate! }),
    enabled: candidate != null,
    staleTime: 0,
    gcTime: 0,
    refetchOnMount: "always",
    retry: false,
  });
  const p: ArenaAssignmentPreviewDto | undefined = preview.isSuccess && !preview.isFetching ? preview.data : undefined;
  const canAssign = !!p && p.allowed && p.table != null && p.record != null && p.field != null;

  const mut = useMutation({
    mutationFn: (plan: ArenaAssignmentPreviewDto) =>
      call("set_field", {
        projectId,
        table: plan.table!,
        record: plan.record!,
        field: plan.field!,
        intValue: Number(plan.to_raw),
      }),
    onSuccess: async (_r, _plan, _ctx) => {
      await qc.invalidateQueries();
      update(await call("project_summary", { projectId }));
    },
  });
  // A second click while the write is in flight is ignored (the mutation's pending flag updates
  // asynchronously, so the guard is a ref).
  const assigningRef = useRef(false);
  const assign = () => {
    if (!canAssign || mut.isPending || assigningRef.current || !p) return;
    const gen = genRef.current;
    assigningRef.current = true;
    mut.mutate(p, {
      onSuccess: () => {
        if (gen !== genRef.current) return; // the cell moved to another target meanwhile
        setCandidate(null);
        setEditing(false);
      },
      onSettled: () => {
        assigningRef.current = false;
      },
    });
  };
  const cancel = () => {
    if (mut.isPending) return;
    setCandidate(null);
    setEditing(false);
  };

  // "Honda Center — Anaheim, CA": StEO's cityname already carries "City, ST".
  const label = (a: { name: string; city: string; arena_id: number | bigint } | null | undefined, raw?: number | bigint | null) =>
    a ? `${a.name || `Arena ${a.arena_id}`}${a.city ? ` — ${a.city}` : ""}` : raw != null ? `Arena ${raw}` : "—";
  const current = arena ? label(arena) : field.decoded != null ? `Arena ${field.decoded}` : "—";
  const sorted = [...arenas].sort((a, b) => a.name.localeCompare(b.name));
  const list = (names: string[]) => names.join(", ");

  return (
    // Spans two grid columns — "Name — City, ST" doesn't fit a single quarter-width cell.
    <div className="col-span-2 min-w-0" data-testid="arena-cell">
      <div className="text-[11px] font-semibold uppercase tracking-wide text-sky-300/80">
        Arena
        {mut.isPending && <span className="ml-1 text-slate-500">…</span>}
      </div>
      <div className="mt-0.5 flex items-center gap-2">
        {editing ? (
          <select
            autoFocus
            className="w-full rounded border border-slate-700 bg-slate-950 px-1.5 py-0.5 font-mono text-xs text-slate-100 focus:border-sky-600 focus:outline-none"
            value={candidate ?? field.decoded ?? 0}
            disabled={mut.isPending}
            onChange={(e) => setCandidate(Number(e.target.value))}
            onKeyDown={(e) => e.key === "Escape" && cancel()}
            data-testid="arena-select"
          >
            {field.decoded != null && !arenas.some((a) => a.arena_id === field.decoded) && (
              <option value={field.decoded}>{`(uncatalogued: ${field.decoded})`}</option>
            )}
            {sorted.map((a) => (
              <option key={a.arena_id} value={a.arena_id}>
                {label(a)}
              </option>
            ))}
          </select>
        ) : (
          <>
            {arena ? (
              <Link
                to={`/app/database/arenas/${arena.arena_id}`}
                className="min-w-0 truncate font-mono text-xs text-sky-400 hover:underline"
                title="Open this arena in the Arenas tab"
              >
                {current}
              </Link>
            ) : (
              <span className="min-w-0 truncate font-mono text-xs text-slate-100">{current}</span>
            )}
            {field.editable && (
              <button
                type="button"
                onClick={() => setEditing(true)}
                title="Change home arena"
                className="shrink-0 rounded border border-slate-700 px-1.5 py-px text-[10px] text-slate-500 hover:border-sky-700 hover:text-sky-300"
                data-testid="arena-change"
              >
                change
              </button>
            )}
          </>
        )}
      </div>
      {editing && candidate != null && (
        <div className="mt-1 rounded border border-slate-800 bg-slate-900/60 p-2 text-[11px]" data-testid="arena-disclosure">
          {preview.isLoading || preview.isFetching ? (
            <p className="text-slate-400" data-testid="arena-disclosure-loading">Checking who plays there…</p>
          ) : preview.error ? (
            <p className="text-amber-300" data-testid="arena-disclosure-error">
              The check failed ({String(preview.error)}). Nothing is assigned until it answers.
            </p>
          ) : p && !p.allowed ? (
            <p className="text-rose-300" data-testid="arena-disclosure-refusal">{p.refusal}</p>
          ) : p ? (
            <div className="space-y-0.5 text-slate-300">
              <p>
                <span className="text-slate-100">{p.team_name}</span> moves to{" "}
                <span className="text-slate-100">{label(p.to, p.to_raw)}</span>.
              </p>
              <p data-testid="arena-disclosure-to">
                {p.to_tenants.length === 0
                  ? "No other team plays there."
                  : `${p.to_tenants.length} team${p.to_tenants.length === 1 ? "" : "s"} already play${p.to_tenants.length === 1 ? "s" : ""} there: ${list(p.to_tenants)}.`}
              </p>
              <p data-testid="arena-disclosure-from">
                {p.from || p.from_raw != null
                  ? p.from_becomes_vacant
                    ? `${label(p.from, p.from_raw)} will have no home team afterwards.`
                    : `${label(p.from, p.from_raw)} keeps ${list(p.from_other_tenants)}.`
                  : "The team has no current arena."}
              </p>
              <p className="text-slate-500">Undo restores the previous arena as one step.</p>
            </div>
          ) : null}
          <div className="mt-1.5 flex gap-2">
            <button
              type="button"
              onClick={assign}
              disabled={!canAssign || mut.isPending}
              className="rounded border border-sky-800 bg-sky-950/40 px-2 py-0.5 text-[11px] text-sky-200 hover:bg-sky-900/40 disabled:opacity-40"
              data-testid="arena-assign"
            >
              {mut.isPending ? "Assigning…" : "Assign"}
            </button>
            <button
              type="button"
              onClick={cancel}
              disabled={mut.isPending}
              className="rounded border border-slate-700 px-2 py-0.5 text-[11px] text-slate-300 hover:bg-slate-800 disabled:opacity-40"
              data-testid="arena-cancel"
            >
              Cancel
            </button>
          </div>
        </div>
      )}
      {mut.error != null && (
        <div className="mt-0.5 text-[10px] text-rose-400" data-testid="arena-assign-error">{String(mut.error)}</div>
      )}
    </div>
  );
}

/** A labelled cell for a field the loaded title's schema doesn't carry — shows an explicit
 *  "Not in Selected Game" instead of a blank the user can't tell apart from an empty value. */
export function NotInGame({ label }: { label: string }) {
  return (
    <Labeled label={label}>
      <span
        className="truncate text-xs italic text-slate-600"
        title="This field is not part of the selected game's database."
      >
        Not in Selected Game
      </span>
    </Labeled>
  );
}

/** A read-only labelled value cell (for resolved/derived fields that aren't a single editable field). */
export function ReadonlyField({ label, children }: { label: string; children?: ReactNode }) {
  const empty = children == null || children === "";
  return (
    <Labeled label={label}>
      <span className="truncate text-xs text-slate-100">
        {empty ? <span className="text-slate-600">—</span> : children}
      </span>
    </Labeled>
  );
}

function Labeled({
  label,
  pending,
  error,
  children,
}: {
  label: string;
  pending?: boolean;
  error?: unknown;
  children: ReactNode;
}) {
  return (
    <div className="min-w-0">
      <div className="text-[11px] font-semibold uppercase tracking-wide text-sky-300/80">
        {label}
        {pending && <span className="ml-1 text-slate-500">…</span>}
      </div>
      <div className="mt-0.5">{children}</div>
      {error != null && <div className="mt-0.5 text-[10px] text-rose-400">{String(error)}</div>}
    </div>
  );
}

const CONTROL =
  "w-full rounded border border-slate-700 bg-slate-950 px-1.5 py-0.5 font-mono text-xs text-slate-100 focus:border-sky-600 focus:outline-none";

function Checkbox({ checked, onChange }: { checked: boolean; onChange: (on: boolean) => void }) {
  return (
    <input
      type="checkbox"
      checked={checked}
      onChange={(e) => onChange(e.target.checked)}
      className="h-3.5 w-3.5 rounded border-slate-600 bg-slate-950 accent-sky-600"
    />
  );
}

function TextNumber({
  field,
  onCommit,
  onRefuse,
  tone,
}: {
  field: FieldViewDto;
  onCommit: (v: { intValue?: number; textValue?: string }) => void;
  /** A pre-write refusal (text over the field's byte width), shown by the owner. */
  onRefuse?: (message: string) => void;
  tone?: string;
}) {
  // The input holds the friendly value (2011, 209, 6'1"); commit converts back to the raw encoding.
  const baseline = friendlySeed(field);
  const [draft, setDraft] = useState(baseline);
  useEffect(() => setDraft(baseline), [baseline]);
  // Inline strings have a fixed byte width; the writer refuses longer text after the fact, so the
  // editor says so first (UTF-8 bytes, not characters — "Litoměřice" is 12 bytes).
  const byteLength = (text: string) => new TextEncoder().encode(text).length;

  const commit = () => {
    if (draft === baseline) return;
    if (field.is_string) {
      const max = field.text_max_bytes;
      if (max != null && byteLength(draft) > max) {
        onRefuse?.(`${byteLength(draft)} bytes — this field holds at most ${max}; shorten the text (accented letters take two bytes). Nothing was written`);
        setDraft(baseline);
        return;
      }
      onCommit({ textValue: draft });
      return;
    }
    const raw = friendlyToRaw(field, draft);
    if (raw == null) {
      setDraft(baseline); // unparseable → revert
      return;
    }
    onCommit({ intValue: clampRaw(field, raw) });
  };
  const bounded = hasPlainBounds(field);
  // A friendly numeric field (weight/height/year/rating) already shows the front value; a plain
  // enhanced-display field (none currently) would show its resolved form as a hint.
  const showHint =
    !field.is_string &&
    field.edit_offset == null &&
    !field.edit_height &&
    field.display !== String(field.decoded ?? "");

  return (
    <div className="flex items-center gap-2">
      <input
        // Height is `ft'in"` text; everything else is numeric.
        type={field.is_string || field.edit_height ? "text" : "number"}
        min={bounded ? field.range_low ?? undefined : undefined}
        max={bounded ? field.range_high ?? undefined : undefined}
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => e.key === "Enter" && commit()}
        className={`${CONTROL} ${tone ? `${tone} font-semibold` : ""}`}
        data-testid="text-number-input"
      />
      {showHint && <span className="shrink-0 text-xs text-slate-400">{field.display}</span>}
    </div>
  );
}

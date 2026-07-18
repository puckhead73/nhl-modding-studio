import { useEffect, useRef, useState, type ReactNode } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { clampRaw, friendlySeed, friendlyToRaw, hasPlainBounds } from "../lib/editForm";
import type { FieldViewDto } from "../bindings/FieldViewDto";

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
  field,
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
  const qc = useQueryClient();
  const { update } = useProject();
  const mut = useMutation({
    mutationFn: (v: { intValue?: number; textValue?: string }) =>
      call("set_field", {
        projectId,
        table: field.studio_table,
        record,
        field: field.studio_field,
        ...v,
      }),
    onSuccess: async () => {
      await qc.invalidateQueries();
      update(await call("project_summary", { projectId }));
    },
  });

  const isFlag =
    !field.is_string && field.range_low === 0 && field.range_high === 1 && field.enum_options.length === 0;

  let control: ReactNode;
  if (!field.editable) {
    control = <span className={`text-xs ${tone ?? "text-slate-100"}`}>{field.display || "—"}</span>;
  } else if (field.enum_options.length > 0) {
    control = (
      <ClickToEdit display={field.display}>
        <Select
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
        <TextNumber field={field} tone={tone} onCommit={(v) => mut.mutate(v)} />
      </ClickToEdit>
    );
  }

  return (
    <Labeled label={label} pending={mut.isPending} error={mut.error}>
      {control}
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
  field,
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
  const qc = useQueryClient();
  const { update } = useProject();
  const mut = useMutation({
    mutationFn: (raw: number) =>
      call("set_field", {
        projectId,
        table: field!.studio_table,
        record,
        field: field!.studio_field,
        intValue: raw,
      }),
    onSuccess: async () => {
      await qc.invalidateQueries();
      update(await call("project_summary", { projectId }));
    },
  });

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

function Select({
  value,
  options,
  onChange,
}: {
  value: number;
  options: { raw: number; label: string }[];
  onChange: (raw: number) => void;
}) {
  // If the current raw isn't in the vocabulary, keep it selectable so we don't silently change it.
  const known = options.some((o) => o.raw === value);
  return (
    <select className={CONTROL} value={value} onChange={(e) => onChange(Number(e.target.value))}>
      {!known && <option value={value}>{`(uncatalogued: ${value})`}</option>}
      {options.map((o) => (
        <option key={o.raw} value={o.raw}>
          {o.label}
        </option>
      ))}
    </select>
  );
}

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
  tone,
}: {
  field: FieldViewDto;
  onCommit: (v: { intValue?: number; textValue?: string }) => void;
  tone?: string;
}) {
  // The input holds the friendly value (2011, 209, 6'1"); commit converts back to the raw encoding.
  const baseline = friendlySeed(field);
  const [draft, setDraft] = useState(baseline);
  useEffect(() => setDraft(baseline), [baseline]);

  const commit = () => {
    if (draft === baseline) return;
    if (field.is_string) {
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
      />
      {showHint && <span className="shrink-0 text-xs text-slate-400">{field.display}</span>}
    </div>
  );
}

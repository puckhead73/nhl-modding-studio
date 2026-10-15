import { useEffect, useState } from "react";
import { useFieldEdit } from "../lib/useFieldEdit";


import { clampRaw, friendlySeed, friendlyToRaw, hasPlainBounds } from "../lib/editForm";
import { ClickToEdit } from "./EditableField";
import { EnumSelect } from "./EnumSelect";
import type { FieldViewDto } from "../bindings/FieldViewDto";

/// One field row: label + value, click-to-edit when the field is updatable (the value reads as
/// plain text until clicked — the reference app's pattern). Commits on Enter or blur (when
/// changed) via `set_field`, then refreshes the open views + dirty flag.
export function FieldRow({
  projectId,
  record,
  field: sourceField,
  labelWidth = "180px",
  label,
}: {
  projectId: number;
  record: number;
  field: FieldViewDto;
  /** Width of the label column. Narrower packs the value closer to the name — used by dense,
   *  half-width panels (e.g. the player Equipment tab) so short labels don't leave a wide gap the
   *  value bleeds across. Defaults to the studio's standard 180px. */
  labelWidth?: string;
  /** Friendly surface label; native field identity remains unchanged for editing. */
  label?: string;
}) {
  const mut = useFieldEdit(projectId, record, sourceField);
  const field = mut.field!;
  // A plain enhanced-display field (none currently — semantic fields carry an edit form) would show
  // its resolved form as a hint beside the raw value.
  const hint =
    !field.is_string &&
    field.edit_offset == null &&
    !field.edit_height &&
    field.display !== String(field.decoded ?? "")
      ? field.display
      : null;
  // Title-scoped vocabularies (25 Sep 2026): a field whose only catalogues belong to other titles
  // says so beside its number instead of borrowing labels; the full reason sits in the tooltip.
  const uncatalogued =
    field.vocabulary_note != null && field.enum_options.length === 0 ? field.vocabulary_note : null;

  return (
    <div
      className="grid min-w-0 items-center gap-2 border-b border-slate-800/60 py-1"
      // `minmax(0, 1fr)` (not `1fr`) lets the value column shrink below its content's intrinsic
      // width so it stays inside the row and truncates, instead of overflowing into the next column.
      style={{ gridTemplateColumns: `${labelWidth} minmax(0, 1fr)` }}
    >
      <span
        className="truncate text-xs text-slate-400"
        title={field.vocabulary_note ? `${field.vocabulary_note} · ${field.studio_field}` : field.studio_field}
      >
        {label ?? field.name}
      </span>
      {field.editable && field.enum_options.length > 0 ? (
        <span className="flex min-w-0 items-center gap-2">
          <span className="w-52 max-w-full">
            <ClickToEdit display={field.display}>
              <EnumSelect
                label={label ?? field.name}
                value={field.decoded ?? 0}
                options={field.enum_options}
                onChange={(raw) => mut.mutate({ intValue: raw })}
              />
            </ClickToEdit>
          </span>
          {mut.isPending && <span className="text-xs text-slate-500">…</span>}
          {mut.isError && <span className="text-xs text-rose-400">{String(mut.error)}</span>}
        </span>
      ) : field.editable ? (
        <span className="flex min-w-0 items-center gap-2">
          <span className="w-52 max-w-full">
            <ClickToEdit display={field.display}>
              <RowInput field={field} onCommit={(payload) => mut.mutate(payload)} />
            </ClickToEdit>
          </span>
          {hint && <span className="text-xs text-slate-400">{hint}</span>}
          {uncatalogued && (
            <span
              className="truncate text-[10px] uppercase tracking-wide text-slate-500"
              title={uncatalogued}
              data-testid="no-catalogue-for-title"
            >
              no catalogue for this title
            </span>
          )}
          {mut.isPending && <span className="text-xs text-slate-500">…</span>}
          {mut.isError && <span className="text-xs text-rose-400">{String(mut.error)}</span>}
        </span>
      ) : (
        <span className="truncate font-mono text-xs text-slate-100" title={field.display}>
          {field.display}
        </span>
      )}
    </div>
  );
}

/// The open-state text/number input. Holds the friendly draft (2011, 209, 6'1") locally, so an
/// Escape-cancel (which unmounts it without a blur) discards the draft instead of leaking it into
/// the next edit; commit converts back to the raw encoding.
function RowInput({
  field,
  onCommit,
}: {
  field: FieldViewDto;
  onCommit: (payload: { intValue?: number; textValue?: string }) => void;
}) {
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

  return (
    <input
      type={field.is_string || field.edit_height ? "text" : "number"}
      min={bounded ? field.range_low ?? undefined : undefined}
      max={bounded ? field.range_high ?? undefined : undefined}
      value={draft}
      onChange={(e) => setDraft(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => e.key === "Enter" && commit()}
      className="w-full rounded border border-slate-700 bg-slate-950 px-1.5 py-0.5 font-mono text-xs text-slate-100 placeholder-slate-600 focus:border-sky-600 focus:outline-none"
    />
  );
}

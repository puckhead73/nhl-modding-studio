import { useEffect, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { clampRaw, friendlySeed, friendlyToRaw, hasPlainBounds } from "../lib/editForm";
import { ClickToEdit } from "./EditableField";
import type { FieldViewDto } from "../bindings/FieldViewDto";

/// One field row: label + value, click-to-edit when the field is updatable (the value reads as
/// plain text until clicked — the reference app's pattern). Commits on Enter or blur (when
/// changed) via `set_field`, then refreshes the open views + dirty flag.
export function FieldRow({
  projectId,
  record,
  field,
}: {
  projectId: number;
  record: number;
  field: FieldViewDto;
}) {
  const qc = useQueryClient();
  const { update } = useProject();
  const mut = useMutation({
    mutationFn: (payload: { intValue?: number; textValue?: string }) =>
      call("set_field", {
        projectId,
        table: field.studio_table,
        record,
        field: field.studio_field,
        ...payload,
      }),
    onSuccess: async () => {
      await qc.invalidateQueries();
      update(await call("project_summary", { projectId }));
    },
  });

  // A plain enhanced-display field (none currently — semantic fields carry an edit form) would show
  // its resolved form as a hint beside the raw value.
  const hint =
    !field.is_string &&
    field.edit_offset == null &&
    !field.edit_height &&
    field.display !== String(field.decoded ?? "")
      ? field.display
      : null;

  return (
    <div className="grid grid-cols-[180px_1fr] items-center gap-2 border-b border-slate-800/60 py-1">
      <span className="text-xs text-slate-400" title={field.studio_field}>
        {field.name}
      </span>
      {field.editable && field.enum_options.length > 0 ? (
        <span className="flex items-center gap-2">
          <span className="w-52 max-w-full">
            <ClickToEdit display={field.display}>
              <select
                value={field.decoded ?? 0}
                onChange={(e) => mut.mutate({ intValue: Number(e.target.value) })}
                className="w-full rounded border border-slate-700 bg-slate-950 px-1.5 py-0.5 font-mono text-xs text-slate-100 focus:border-sky-600 focus:outline-none"
              >
                {!field.enum_options.some((o) => o.raw === field.decoded) && (
                  <option value={field.decoded ?? 0}>{`(uncatalogued: ${field.decoded ?? 0})`}</option>
                )}
                {field.enum_options.map((o) => (
                  <option key={o.raw} value={o.raw}>
                    {o.label}
                  </option>
                ))}
              </select>
            </ClickToEdit>
          </span>
          {mut.isPending && <span className="text-xs text-slate-500">…</span>}
          {mut.isError && <span className="text-xs text-rose-400">{String(mut.error)}</span>}
        </span>
      ) : field.editable ? (
        <span className="flex items-center gap-2">
          <span className="w-52 max-w-full">
            <ClickToEdit display={field.display}>
              <RowInput field={field} onCommit={(payload) => mut.mutate(payload)} />
            </ClickToEdit>
          </span>
          {hint && <span className="text-xs text-slate-400">{hint}</span>}
          {mut.isPending && <span className="text-xs text-slate-500">…</span>}
          {mut.isError && <span className="text-xs text-rose-400">{String(mut.error)}</span>}
        </span>
      ) : (
        <span className="font-mono text-xs text-slate-100">{field.display}</span>
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

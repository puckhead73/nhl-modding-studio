import { useEffect, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import type { FieldViewDto } from "../bindings/FieldViewDto";

/// One field row: label + value, with an inline editor when the field is updatable. Commits on
/// Enter or blur (when changed) via `set_field`, then refreshes the open views + dirty flag.
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
  const baseline = field.is_string ? field.text ?? "" : String(field.decoded ?? 0);
  const [draft, setDraft] = useState(baseline);

  useEffect(() => setDraft(baseline), [baseline]);

  const mut = useMutation({
    mutationFn: (val: string) =>
      call("set_field", {
        projectId,
        table: field.studio_table,
        record,
        field: field.studio_field,
        ...(field.is_string ? { textValue: val } : { intValue: Number(val) }),
      }),
    onSuccess: async () => {
      await qc.invalidateQueries();
      update(await call("project_summary", { projectId }));
    },
  });

  const commit = () => {
    if (draft !== baseline) mut.mutate(draft);
  };

  return (
    <div className="grid grid-cols-[180px_1fr] items-center gap-2 border-b border-slate-800/60 py-1">
      <span className="text-xs text-slate-400" title={field.studio_field}>
        {field.name}
      </span>
      {field.editable ? (
        <span className="flex items-center gap-2">
          <input
            type={field.is_string ? "text" : "number"}
            value={draft}
            min={field.range_low ?? undefined}
            max={field.range_high ?? undefined}
            onChange={(e) => setDraft(e.target.value)}
            onBlur={commit}
            onKeyDown={(e) => e.key === "Enter" && commit()}
            className="w-52 max-w-full rounded border border-slate-700 bg-slate-950 px-2 py-1 text-sm text-slate-100 placeholder-slate-600 focus:border-sky-600 focus:outline-none"
          />
          {field.enum_label && <span className="text-xs text-slate-400">{field.enum_label}</span>}
          {mut.isPending && <span className="text-xs text-slate-500">…</span>}
          {mut.isError && <span className="text-xs text-rose-400">{String(mut.error)}</span>}
        </span>
      ) : (
        <span className="text-slate-100">{field.display}</span>
      )}
    </div>
  );
}

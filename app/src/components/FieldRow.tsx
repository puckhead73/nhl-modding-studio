import { useEffect, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import type { FieldViewDto } from "../bindings/FieldViewDto";
import styles from "./FieldRow.module.css";

/// One field row: label + value, with an inline editor when the field is updatable. Commits on
/// Enter or blur (when changed) via `set_field`, then refreshes the player + summary (dirty flag).
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

  // Re-sync the local draft whenever the underlying value changes (after a refetch).
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
      await qc.invalidateQueries({ queryKey: ["player", projectId] });
      await qc.invalidateQueries({ queryKey: ["players", projectId] });
      update(await call("project_summary", { projectId }));
    },
  });

  const commit = () => {
    if (draft !== baseline) mut.mutate(draft);
  };

  return (
    <div className={styles.row}>
      <span className={styles.label} title={field.studio_field}>
        {field.name}
      </span>
      {field.editable ? (
        <span className={styles.editor}>
          <input
            className={styles.input}
            type={field.is_string ? "text" : "number"}
            value={draft}
            min={field.range_low ?? undefined}
            max={field.range_high ?? undefined}
            onChange={(e) => setDraft(e.target.value)}
            onBlur={commit}
            onKeyDown={(e) => e.key === "Enter" && commit()}
          />
          {field.enum_label && <span className={styles.hint}>{field.enum_label}</span>}
          {mut.isPending && <span className={styles.hint}>…</span>}
          {mut.isError && <span className={styles.err}>{String(mut.error)}</span>}
        </span>
      ) : (
        <span className={styles.readonly}>{field.display}</span>
      )}
    </div>
  );
}

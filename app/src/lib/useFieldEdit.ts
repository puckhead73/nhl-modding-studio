import { useEffect, useRef, useState } from "react";
import { useMutation, useQueryClient, type QueryClient } from "@tanstack/react-query";
import { call } from "./client";
import { useProject } from "./project";
import { friendlySeed } from "./editForm";
import { awaitFieldWrites, queueFieldWrite } from "./fieldWrites";
import type { FieldViewDto } from "../bindings/FieldViewDto";
import type { ProjectSummaryDto } from "../bindings/ProjectSummaryDto";

type Value = { intValue?: number; textValue?: string };
type Refresh = { timer?: ReturnType<typeof setTimeout>; started: boolean; resolve: (() => void)[]; reject: ((reason: unknown) => void)[] };
const refreshes = new WeakMap<QueryClient, Map<number, Refresh>>();

/** One project refresh per burst; unrelated projects and portrait previews stay cached. */
export function refreshFields(qc: QueryClient, projectId: number, update: (summary: ProjectSummaryDto) => void) {
  let pending = refreshes.get(qc);
  if (!pending) { pending = new Map(); refreshes.set(qc, pending); }
  let batch = pending.get(projectId);
  if (batch) clearTimeout(batch.timer);
  else { batch = { started: false, resolve: [], reject: [] }; pending.set(projectId, batch); }
  const current = batch;
  const result = new Promise<void>((resolve, reject) => { current.resolve.push(resolve); current.reject.push(reject); });
  if (current.started) return result;
  current.timer = setTimeout(() => {
    current.started = true;
    void (async () => {
      await awaitFieldWrites(projectId);
      pending!.delete(projectId);
      // Overview is revision-keyed and refreshes when the summary below is published.
      const predicate = (q: { queryKey: readonly unknown[] }) => q.queryKey[1] === projectId && q.queryKey[0] !== "project-overview";
      await Promise.all([
        qc.invalidateQueries({ predicate }),
        call("project_summary", { projectId }).then(update),
      ]);
    })().then(() => current.resolve.forEach(resolve => resolve()), error => current.reject.forEach(reject => reject(error)));
  }, 120);
  return result;
}

/** Echo a submitted field in its display units while the validated backend edit runs. */
export function pendingField(field: FieldViewDto, value: Value): FieldViewDto {
  if (value.textValue != null) return { ...field, text: value.textValue, display: value.textValue };
  const raw = value.intValue!;
  const next = { ...field, decoded: raw, raw_int: raw };
  const label = field.enum_options.find(option => option.raw === raw)?.label;
  const friendly = friendlySeed(next);
  const display = label ?? (field.display.startsWith("$") && field.edit_scale != null
    ? `$${Number(friendly).toLocaleString("en-US")}`
    : field.display.endsWith(" lb") ? `${friendly} lb` : friendly);
  return { ...next, display, enum_label: label ?? null };
}

export function useFieldEdit(projectId: number, record: number, field: FieldViewDto | undefined) {
  const qc = useQueryClient();
  const { update } = useProject();
  const target = `${projectId}/${field?.studio_table}/${record}/${field?.studio_field}`;
  const ticket = useRef(0);
  const accepted = useRef<{ target: string; field: FieldViewDto } | null>(null);
  const [echo, setEcho] = useState<{ target: string; ticket: number; field: FieldViewDto } | null>(null);
  const [refreshError, setRefreshError] = useState<{ target: string; text: string } | null>(null);
  useEffect(() => {
    // Undo, reload and navigation replace the confirmed value used for a later rollback.
    if (field && (!echo || echo.target !== target)) accepted.current = { target, field };
  }, [field, target, echo]);
  const mutation = useMutation({
    mutationFn: (edit: { ticket: number; target: string; value: Value; source: FieldViewDto }) => queueFieldWrite(projectId, () => call("set_field", {
      projectId, table: edit.source.studio_table, record, field: edit.source.studio_field, ...edit.value,
    })),
    onSuccess: (_, edit) => {
      accepted.current = { target: edit.target, field: pendingField(edit.source, edit.value) };
      // Do not make the next write wait for lists, validation and summary rendering.
      void refreshFields(qc, projectId, update).then(() => {
        setEcho(current => current?.target === edit.target && current.ticket === edit.ticket ? null : current);
      }, error => setRefreshError({ target: edit.target, text: `Edit applied, but the view could not refresh: ${String(error)}` }));
    },
    onError: (_, edit) => {
      setEcho(current => current?.target === edit.target && current.ticket === edit.ticket
        ? accepted.current?.target === edit.target ? { ...current, field: accepted.current.field } : null
        : current);
      void refreshFields(qc, projectId, update).then(() => {
        setEcho(current => current?.target === edit.target && current.ticket === edit.ticket ? null : current);
      }).catch(() => {});
    },
  });
  return {
    ...mutation,
    error: (mutation.variables?.target === target ? mutation.error : null) ?? (refreshError?.target === target ? refreshError.text : null),
    field: echo?.target === target ? echo.field : field,
    mutate: (value: Value) => {
      if (!field) return;
      const next = ++ticket.current;
      setRefreshError(null);
      setEcho({ target, ticket: next, field: pendingField(field, value) });
      mutation.mutate({ ticket: next, target, value, source: field });
    },
  };
}

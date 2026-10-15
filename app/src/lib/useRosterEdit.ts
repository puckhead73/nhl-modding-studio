import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useProject } from "./project";
import { queueFieldWrite } from "./fieldWrites";
import { refreshFields } from "./useFieldEdit";
import type { LinesViewDto } from "../bindings/LinesViewDto";

/** Show roster changes immediately; rejected writes restore the confirmed snapshot. */
export function useRosterEdit<V>(projectId: number, teamId: number,
  write: (value: V, before: LinesViewDto | undefined) => Promise<unknown>,
  project: (before: LinesViewDto, value: V) => LinesViewDto) {
  const qc = useQueryClient();
  const { update } = useProject();
  const key = ["lines", projectId, teamId];
  const mutation = useMutation({
    mutationFn: ({ value, before }: { value: V; before: LinesViewDto | undefined }) =>
      queueFieldWrite(projectId, () => write(value, before)),
    onMutate: async ({ value }) => {
      await qc.cancelQueries({ queryKey: key });
      const before = qc.getQueryData<LinesViewDto>(key);
      if (before) qc.setQueryData(key, project(before, value));
      return before;
    },
    onError: (_error, _value, before) => { if (before) qc.setQueryData(key, before); },
    onSettled: () => { void refreshFields(qc, projectId, update).catch(() => {}); },
  });
  return { ...mutation, mutate: (value: V) => mutation.mutate({ value, before: qc.getQueryData<LinesViewDto>(key) }) };
}

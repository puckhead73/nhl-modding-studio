import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { MasterList, type ListItem } from "../components/MasterList";

export function ArenasPage() {
  const { project } = useProject();
  const projectId = project!.project_id;

  const arenas = useQuery({
    queryKey: ["arenas", projectId],
    queryFn: () => call("list_arenas", { projectId }),
  });

  const items: ListItem[] = useMemo(
    () =>
      (arenas.data ?? []).map((a) => ({
        id: a.arena_id,
        primary: a.name || `Arena ${a.arena_id}`,
        secondary: a.city,
      })),
    [arenas.data],
  );

  return (
    <MasterList
      items={items}
      isLoading={arenas.isLoading}
      error={arenas.error}
      placeholder="Filter arenas…"
    />
  );
}

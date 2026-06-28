import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { MasterList, type ListItem } from "../components/MasterList";

export function TeamsPage() {
  const { project } = useProject();
  const projectId = project!.project_id;

  const teams = useQuery({
    queryKey: ["teams", projectId],
    queryFn: () => call("list_teams", { projectId }),
  });

  const items: ListItem[] = useMemo(
    () =>
      (teams.data ?? []).map((t) => ({
        id: t.team_id,
        primary: t.name || `Team ${t.team_id}`,
        secondary: t.abbrev,
      })),
    [teams.data],
  );

  return (
    <MasterList
      items={items}
      isLoading={teams.isLoading}
      error={teams.error}
      placeholder="Filter teams…"
    />
  );
}

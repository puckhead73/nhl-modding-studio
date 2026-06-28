import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { MasterList, type ListItem } from "../components/MasterList";

export function PlayersPage() {
  const { project } = useProject();
  const projectId = project!.project_id;

  const players = useQuery({
    queryKey: ["players", projectId],
    queryFn: () => call("list_players", { projectId }),
  });

  const items: ListItem[] = useMemo(
    () =>
      (players.data ?? []).map((p) => ({
        id: p.player_id,
        primary: `${p.last_name || "—"}${p.first_name ? `, ${p.first_name}` : ""}`,
        secondary: p.position_label,
      })),
    [players.data],
  );

  return (
    <MasterList
      items={items}
      isLoading={players.isLoading}
      error={players.error}
      placeholder="Filter players…"
    />
  );
}

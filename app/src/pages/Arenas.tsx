import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { EntityList, type Column } from "../components/EntityList";
import { EmptyState } from "../components/primitives";
import { isAvailable } from "../lib/surfaces";

type Row = { id: number; name: string; city: string; teams: number };

export function ArenasPage() {
  const { project } = useProject();
  const projectId = project!.project_id;
  // Direct-URL safety: a title whose schema has no named arena table (e.g. NHL 07) shows an explicit
  // unavailable state instead of a name-less list — and never fires the query.
  const arenasAvailable = isAvailable(project!.surfaces?.arenas);

  const arenas = useQuery({
    queryKey: ["arenas", projectId],
    queryFn: () => call("list_arenas", { projectId }),
    enabled: arenasAvailable,
  });

  const rows: Row[] = useMemo(
    () =>
      (arenas.data ?? []).map((a) => ({
        id: a.arena_id,
        name: a.name || `Arena ${a.arena_id}`,
        city: a.city,
        teams: a.team_count,
      })),
    [arenas.data],
  );

  if (!arenasAvailable) {
    return (
      <EmptyState
        title="Arenas aren’t available for this title"
        hint={project!.surfaces?.arenas.reason ?? "This game's database doesn't carry a named arena table."}
      />
    );
  }

  const columns: Column<Row>[] = [
    {
      key: "id",
      label: "ID",
      width: "40px",
      sort: (r) => r.id,
      render: (r) => r.id,
      cellClass: () => "tabular-nums text-slate-500",
    },
    {
      key: "name",
      label: "Name",
      width: "1fr",
      sort: (r) => r.name,
      render: (r) => r.name,
      cellClass: () => "font-medium",
    },
    {
      key: "city",
      label: "City",
      width: "104px",
      sort: (r) => r.city,
      render: (r) => r.city,
      cellClass: () => "text-slate-400",
    },
    {
      key: "teams",
      label: "Teams",
      width: "56px",
      sort: (r) => r.teams,
      render: (r) => (r.teams > 0 ? r.teams : "—"),
      cellClass: (r) => (r.teams > 0 ? "tabular-nums text-slate-300" : "text-slate-600"),
    },
  ];

  return (
    <EntityList
      title="Arenas"
      rows={rows}
      getId={(r) => r.id}
      columns={columns}
      searchOf={(r) => `${r.name} ${r.city} ${r.id}`}
      searchPlaceholder="Search name / city / id…"
      isLoading={arenas.isLoading}
      error={arenas.error}
      initialSort="id"
      // Arena names run long ("Scotiabank Saddledome") — a wider panel keeps them un-truncated
      // next to the City and Teams columns.
      panelWidth="w-[27rem]"
    />
  );
}

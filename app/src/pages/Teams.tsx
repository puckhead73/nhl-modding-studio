import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { EntityList, type Column } from "../components/EntityList";

type Row = { id: number; name: string; abbrev: string };

export function TeamsPage() {
  const { project } = useProject();
  const projectId = project!.project_id;

  const teams = useQuery({
    queryKey: ["teams", projectId],
    queryFn: () => call("list_teams", { projectId }),
  });

  const rows: Row[] = useMemo(
    () =>
      (teams.data ?? []).map((t) => ({
        id: t.team_id,
        name: t.name || `Team ${t.team_id}`,
        abbrev: t.abbrev,
      })),
    [teams.data],
  );

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
      key: "abbrev",
      label: "Abbr",
      width: "52px",
      sort: (r) => r.abbrev,
      render: (r) => r.abbrev,
      cellClass: () => "text-teal-300/80 font-medium",
    },
  ];

  return (
    <EntityList
      title="Teams"
      rows={rows}
      getId={(r) => r.id}
      columns={columns}
      searchOf={(r) => `${r.name} ${r.abbrev} ${r.id}`}
      searchPlaceholder="Search name / abbr / id…"
      isLoading={teams.isLoading}
      error={teams.error}
      initialSort="id"
    />
  );
}

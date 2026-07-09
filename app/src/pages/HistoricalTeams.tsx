import { useState } from "react";
import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { EntityList, type Column } from "../components/EntityList";
import type { HistoricalTeamIndexEntryDto } from "../bindings/HistoricalTeamIndexEntryDto";

const LIMIT = 2000;

type Row = HistoricalTeamIndexEntryDto & { span: string };

function spanLabel(first: number | null, last: number | null): string {
  if (first == null && last == null) return "—";
  if (first === last) return `${first}`;
  return `${first ?? "?"}–${last ?? "?"}`;
}

/// Historical Teams — franchise-consolidated. NHL relocations/renames collapse into one franchise
/// entry (the v5 lineage); every non-NHL club stays standalone. The top filter is by **league**
/// (`NHL` lists the 30-odd franchises; another league lists its teams; "All" unions everything).
/// Read-only. Selecting an entry opens its per-season detail.
export function HistoricalTeamsPage() {
  const filters = useQuery({
    queryKey: ["hist_team_filters"],
    queryFn: () => call("historical_team_filters", {}),
  });
  // Default to the NHL franchises — the headline of the consolidated view.
  const [league, setLeague] = useState<string>("NHL");
  const [search, setSearch] = useState("");

  const teams = useQuery({
    queryKey: ["hist_franchises", league, search],
    queryFn: () =>
      call("list_historical_franchises", {
        league: league === "" ? null : league,
        search,
        limit: LIMIT,
      }),
    placeholderData: keepPreviousData,
  });

  const rows: Row[] = (teams.data ?? []).map((t) => ({
    ...t,
    span: spanLabel(t.first_year, t.last_year),
  }));

  const columns: Column<Row>[] = [
    {
      key: "name",
      label: "Name",
      width: "1fr",
      sort: (r) => r.name,
      render: (r) => (
        <span>
          {r.name}
          {r.is_defunct ? <span className="ml-1 text-[10px] text-slate-500">(defunct)</span> : null}
        </span>
      ),
      cellClass: () => "font-medium",
    },
    {
      key: "abbr",
      label: "Abbr",
      width: "56px",
      sort: (r) => r.abbr ?? "",
      render: (r) => r.abbr ?? "",
      cellClass: () => "text-teal-300/80 font-medium",
    },
    {
      key: "league",
      label: "Lg",
      width: "56px",
      sort: (r) => r.league_abbr ?? "",
      render: (r) => r.league_abbr ?? "",
      cellClass: () => "text-slate-400 truncate",
    },
    {
      key: "span",
      label: "Seasons",
      width: "84px",
      sort: (r) => r.first_year ?? 0,
      render: (r) => r.span,
      cellClass: () => "text-slate-500 tabular-nums",
    },
  ];

  return (
    <EntityList
      title="Historical Teams"
      rows={rows}
      total={rows.length}
      getId={() => 0}
      getKey={(r) => `${r.kind}~${r.key}`}
      columns={columns}
      searchOf={(r) => r.name}
      searchPlaceholder="Search team…"
      isLoading={teams.isLoading || filters.isLoading}
      error={teams.error ?? filters.error}
      initialSort="name"
      onSearchChange={setSearch}
      panelWidth="w-96"
      toolbar={
        <select
          value={league}
          onChange={(e) => setLeague(e.target.value)}
          className="rounded-md border border-slate-700 bg-slate-950 px-2 py-1.5 text-xs text-slate-100 focus:border-sky-600 focus:outline-none"
        >
          <option value="NHL">NHL</option>
          <option value="">All leagues</option>
          {(filters.data?.leagues ?? [])
            .filter((l) => l.abbr !== "NHL")
            .map((l) => (
              <option key={l.abbr} value={l.abbr}>
                {l.abbr}
              </option>
            ))}
        </select>
      }
    />
  );
}

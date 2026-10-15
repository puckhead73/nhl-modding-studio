import { useState } from "react";
import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { EntityList, type Column } from "../components/EntityList";
import { routeKey } from "../lib/legendsKey";

const LIMIT = 2000;

/** A list row: a built Legends team, or a bare franchise when the ratings aren't computed yet. */
type Row = {
  /** The backend team key (`franchise:<id>` / a residual key). */
  key: string;
  name: string;
  abbr: string | null;
  isSpecial: boolean;
  /** Roster counts — null until the Legends ratings have been calculated. */
  forwards: number | null;
  defense: number | null;
  goalies: number | null;
};

/// Puck's Picks — the first Legends build strategy. The master list is every franchise that has
/// played in the NHL, consolidated the same way the Historical Teams view consolidates them
/// (relocations/renames collapse into one franchise entry). The corpus's NHL view *is* exactly the
/// franchise set — every NHL `team_key` is a franchise member — so this is the `NHL` league filter
/// with no league picker: a Legends build always targets an NHL franchise.
///
/// Picking one opens its builder, where the Season Identity dropdown sets which era's colours, name
/// and (later) logo the built team wears. Roster population from calculated OVR lands next.
export function LegendsPicksPage() {
  const [search, setSearch] = useState("");

  // The built teams — available only once the active profile's Legends ratings exist.
  const built = useQuery({
    queryKey: ["legends_teams"],
    queryFn: () => call("legends_teams", {}),
    retry: false,
    // The built league only changes when the ratings are recalculated, and that path invalidates
    // this key explicitly — so never refetch it on remount while navigating the section.
    staleTime: Infinity,
  });
  // The franchise list always resolves, so the page still shows what Puck's Picks *will* build
  // before any calculation has been run.
  const franchises = useQuery({
    queryKey: ["legends_franchises", search],
    queryFn: () => call("list_historical_franchises", { league: "NHL", search, limit: LIMIT }),
    enabled: built.isError || built.data == null,
    placeholderData: keepPreviousData,
  });

  const needle = search.trim().toLowerCase();
  const rows: Row[] = built.data
    ? built.data
        .filter((t) => !needle || t.name.toLowerCase().includes(needle))
        .map((t) => ({
          key: t.key,
          name: t.name,
          abbr: t.abbr,
          isSpecial: t.is_special,
          forwards: t.forwards,
          defense: t.defense,
          goalies: t.goalies,
        }))
    : (franchises.data ?? []).map((t) => ({
        key: `franchise:${t.key}`,
        name: t.name,
        abbr: t.abbr,
        isSpecial: false,
        forwards: null,
        defense: null,
        goalies: null,
      }));

  const columns: Column<Row>[] = [
    {
      key: "name",
      label: "Team",
      width: "1fr",
      sort: (r) => r.name,
      render: (r) => (
        <span>
          {r.name}
          {r.isSpecial && (
            <span className="ml-1 text-[10px] text-amber-400/80" title="A Legends-only team">
              (legends)
            </span>
          )}
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
      key: "roster",
      label: "F / D / G",
      width: "84px",
      sort: (r) => (r.forwards ?? 0) + (r.defense ?? 0) + (r.goalies ?? 0),
      render: (r) =>
        r.forwards == null ? "—" : `${r.forwards}/${r.defense}/${r.goalies}`,
      cellClass: () => "text-slate-500 tabular-nums",
    },
  ];

  return (
    <EntityList
      title="Puck's Picks"
      rows={rows}
      total={rows.length}
      getId={() => 0}
      getKey={(r) => routeKey(r.key)}
      columns={columns}
      searchOf={(r) => r.name}
      searchPlaceholder="Search team…"
      isLoading={built.isLoading || franchises.isLoading}
      error={franchises.error}
      initialSort="name"
      onSearchChange={setSearch}
      panelWidth="w-96"
    />
  );
}

import { useState } from "react";
import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { EntityList, type Column } from "../components/EntityList";
import { POS_BADGE } from "../lib/position";

type Row = {
  id: number;
  name: string;
  pos: string;
  seasons: number;
  nat: string;
};

const LIMIT = 500;

/// Historical Players — the person-centric master list over the FHM corpus. Because the corpus holds
/// 150k+ people, the list is server-filtered: the search box drives a debounced query (via
/// `EntityList`'s `onSearchChange`) rather than dumping every row to the client. Read-only.
export function HistoricalPlayersPage() {
  const [search, setSearch] = useState("");
  const [posMode, setPosMode] = useState<"all" | "skaters" | "goalies">("all");
  const goalieFilter = posMode === "all" ? null : posMode === "goalies";

  const players = useQuery({
    queryKey: ["hist_players", search, goalieFilter],
    queryFn: () => call("list_historical_players", { search, goalieFilter, limit: LIMIT }),
    placeholderData: keepPreviousData,
  });

  const rows: Row[] = (players.data ?? []).map((p) => ({
    id: p.person_id,
    name: `${p.last_name}, ${p.first_name}`,
    pos: p.position ?? (p.is_goalie ? "G" : ""),
    seasons: p.seasons_played,
    nat: p.nationality ?? "",
  }));

  const capped = rows.length >= LIMIT;

  const columns: Column<Row>[] = [
    {
      key: "name",
      label: "Name",
      width: "1fr",
      sort: (r) => r.name,
      render: (r) => r.name,
      cellClass: () => "font-medium",
    },
    {
      key: "pos",
      label: "Pos",
      width: "44px",
      sort: (r) => r.pos,
      render: (r) =>
        r.pos ? (
          <span
            className={
              "inline-flex items-center rounded px-1 py-0.5 text-[10px] font-semibold " +
              (POS_BADGE[r.pos] ?? "bg-slate-800 text-slate-300")
            }
          >
            {r.pos}
          </span>
        ) : null,
    },
    {
      key: "seasons",
      label: "Seasons",
      width: "88px",
      sort: (r) => r.seasons,
      render: (r) => r.seasons,
      cellClass: () => "tabular-nums text-slate-400",
    },
    {
      key: "nat",
      label: "Nat",
      width: "72px",
      sort: (r) => r.nat,
      render: (r) => r.nat,
      cellClass: () => "text-slate-400 truncate",
    },
  ];

  return (
    <EntityList
      title="Historical Players"
      rows={rows}
      total={rows.length}
      getId={(r) => r.id}
      columns={columns}
      searchOf={(r) => r.name}
      searchPlaceholder={capped ? `Search rated players (top ${LIMIT} shown)…` : "Search name…"}
      isLoading={players.isLoading}
      error={players.error}
      initialSort="name"
      onSearchChange={setSearch}
      panelWidth="w-96"
      toolbar={
        <select
          value={posMode}
          onChange={(e) => setPosMode(e.target.value as typeof posMode)}
          className="rounded-md border border-slate-700 bg-slate-950 px-2 py-1.5 text-xs text-slate-100 focus:border-sky-600 focus:outline-none"
        >
          <option value="all">All</option>
          <option value="skaters">Skaters</option>
          <option value="goalies">Goalies</option>
        </select>
      }
    />
  );
}

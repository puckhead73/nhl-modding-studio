import { useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { EntityList, type Column } from "../components/EntityList";
import { POS_BADGE } from "../lib/position";

type Row = { id: number; name: string; pos: string; team: string; isFa: boolean; badName: boolean };

// A name that failed to decode shows the Unicode replacement char (U+FFFD, from `from_utf8_lossy`)
// or a literal "?" — a byte the game's text encoding couldn't map. Flag those so the modder can find
// and fix them; they're pinned to the bottom of the list.
const UNDECODABLE = /[�?]/;

export function PlayersPage() {
  const { project } = useProject();
  const projectId = project!.project_id;

  const players = useQuery({
    queryKey: ["players", projectId],
    queryFn: () => call("list_players", { projectId }),
  });
  // Teams give us each player's primary-team abbreviation (the summary carries only team_id).
  const teams = useQuery({
    queryKey: ["teams", projectId],
    queryFn: () => call("list_teams", { projectId }),
  });

  const abbrevOf = useMemo(() => {
    const m = new Map<number, string>();
    for (const t of teams.data ?? []) m.set(t.team_id, t.abbrev || t.name);
    return m;
  }, [teams.data]);

  const allRows: Row[] = useMemo(
    () =>
      (players.data ?? []).map((p) => {
        const isFa = p.team_id == null;
        const name = `${p.last_name || "—"}${p.first_name ? `, ${p.first_name}` : ""}`;
        return {
          id: p.player_id,
          name,
          pos: p.position_label,
          team: isFa ? "FA" : abbrevOf.get(p.team_id!) ?? `#${p.team_id}`,
          isFa,
          badName: UNDECODABLE.test(name),
        };
      }),
    [players.data, abbrevOf],
  );

  const positions = useMemo(() => {
    const s = new Set<string>();
    for (const r of allRows) if (r.pos) s.add(r.pos);
    return [...s].sort();
  }, [allRows]);

  const [pos, setPos] = useState("All");
  const rows = useMemo(
    () => (pos === "All" ? allRows : allRows.filter((r) => r.pos === pos)),
    [allRows, pos],
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
      render: (r) =>
        r.badName ? (
          <span title="Name contains a character that failed to decode">{r.name} ⚠</span>
        ) : (
          r.name
        ),
      cellClass: (r) => (r.badName ? "font-medium text-amber-400" : "font-medium"),
    },
    {
      key: "pos",
      label: "Pos",
      width: "40px",
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
      key: "team",
      label: "Team",
      width: "48px",
      sort: (r) => r.team,
      render: (r) => r.team,
      cellClass: (r) => (r.isFa ? "text-amber-400 font-medium" : "text-teal-300/80 font-medium"),
    },
  ];

  return (
    <EntityList
      title="Players"
      rows={rows}
      total={allRows.length}
      getId={(r) => r.id}
      columns={columns}
      searchOf={(r) => `${r.name} ${r.team} ${r.id}`}
      searchPlaceholder="Search name / team / id…"
      isLoading={players.isLoading}
      error={players.error}
      initialSort="id"
      pinLast={(r) => r.badName}
      toolbar={
        <select
          value={pos}
          onChange={(e) => setPos(e.target.value)}
          className="rounded-md border border-slate-700 bg-slate-950 px-2 py-1.5 text-xs text-slate-100 focus:border-sky-600 focus:outline-none"
        >
          <option value="All">All</option>
          {positions.map((p) => (
            <option key={p} value={p}>
              {p}
            </option>
          ))}
        </select>
      }
    />
  );
}

import { useMemo, useState } from "react";
import { NavLink, Outlet } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import styles from "./Players.module.css";

export function PlayersPage() {
  const { project } = useProject();
  const projectId = project!.project_id;
  const [filter, setFilter] = useState("");

  const players = useQuery({
    queryKey: ["players", projectId],
    queryFn: () => call("list_players", { projectId }),
  });

  const rows = useMemo(() => {
    const all = players.data ?? [];
    if (!filter.trim()) return all;
    const q = filter.toLowerCase();
    return all.filter(
      (p) =>
        p.last_name.toLowerCase().includes(q) ||
        p.first_name.toLowerCase().includes(q),
    );
  }, [players.data, filter]);

  return (
    <div className={styles.split}>
      <aside className={styles.list}>
        <input
          className={styles.filter}
          placeholder="Filter by name…"
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
        />
        {players.isLoading && <p className={styles.muted}>Loading players…</p>}
        {players.isError && <p className={styles.err}>{String(players.error)}</p>}
        {players.data && (
          <p className={styles.muted}>
            {rows.length} of {players.data.length}
          </p>
        )}
        <div className={styles.rows}>
          {rows.slice(0, 1000).map((p) => (
            <NavLink
              key={p.player_id}
              to={String(p.player_id)}
              className={({ isActive }) => (isActive ? styles.rowActive : styles.row)}
            >
              <span className={styles.name}>
                {p.last_name || "—"}
                {p.first_name ? `, ${p.first_name}` : ""}
              </span>
              <span className={styles.pos}>{p.position_label}</span>
            </NavLink>
          ))}
          {rows.length > 1000 && (
            <p className={styles.muted}>Showing first 1000 — refine the filter.</p>
          )}
        </div>
      </aside>
      <section className={styles.detail}>
        <Outlet />
      </section>
    </div>
  );
}

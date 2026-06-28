import { useState, useMemo } from "react";
import { NavLink, Outlet } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import styles from "./list.module.css";

export function TeamsPage() {
  const { project } = useProject();
  const projectId = project!.project_id;
  const [filter, setFilter] = useState("");

  const teams = useQuery({
    queryKey: ["teams", projectId],
    queryFn: () => call("list_teams", { projectId }),
  });

  const rows = useMemo(() => {
    const all = teams.data ?? [];
    if (!filter.trim()) return all;
    const q = filter.toLowerCase();
    return all.filter((t) => t.name.toLowerCase().includes(q) || t.abbrev.toLowerCase().includes(q));
  }, [teams.data, filter]);

  return (
    <div className={styles.split}>
      <aside className={styles.list}>
        <input
          className={styles.filter}
          placeholder="Filter teams…"
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
        />
        {teams.isLoading && <p className={styles.muted}>Loading teams…</p>}
        {teams.isError && <p className={styles.err}>{String(teams.error)}</p>}
        <div className={styles.rows}>
          {rows.map((t) => (
            <NavLink
              key={t.team_id}
              to={String(t.team_id)}
              className={({ isActive }) => (isActive ? styles.rowActive : styles.row)}
            >
              <span className={styles.name}>{t.name || `Team ${t.team_id}`}</span>
              <span className={styles.sub}>{t.abbrev}</span>
            </NavLink>
          ))}
        </div>
      </aside>
      <section className={styles.detail}>
        <Outlet />
      </section>
    </div>
  );
}

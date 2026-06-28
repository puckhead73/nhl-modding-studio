import { useState, useMemo } from "react";
import { NavLink, Outlet } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import styles from "./list.module.css";

export function ArenasPage() {
  const { project } = useProject();
  const projectId = project!.project_id;
  const [filter, setFilter] = useState("");

  const arenas = useQuery({
    queryKey: ["arenas", projectId],
    queryFn: () => call("list_arenas", { projectId }),
  });

  const rows = useMemo(() => {
    const all = arenas.data ?? [];
    if (!filter.trim()) return all;
    const q = filter.toLowerCase();
    return all.filter((a) => a.name.toLowerCase().includes(q) || a.city.toLowerCase().includes(q));
  }, [arenas.data, filter]);

  return (
    <div className={styles.split}>
      <aside className={styles.list}>
        <input
          className={styles.filter}
          placeholder="Filter arenas…"
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
        />
        {arenas.isLoading && <p className={styles.muted}>Loading arenas…</p>}
        {arenas.isError && <p className={styles.err}>{String(arenas.error)}</p>}
        <div className={styles.rows}>
          {rows.map((a) => (
            <NavLink
              key={a.arena_id}
              to={String(a.arena_id)}
              className={({ isActive }) => (isActive ? styles.rowActive : styles.row)}
            >
              <span className={styles.name}>{a.name || `Arena ${a.arena_id}`}</span>
              <span className={styles.sub}>{a.city}</span>
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

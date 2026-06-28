import { NavLink, Navigate, Outlet, useNavigate } from "react-router-dom";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useProject } from "./lib/project";
import { call } from "./lib/client";
import type { ProjectSummaryDto } from "./bindings/ProjectSummaryDto";
import styles from "./AppShell.module.css";

const NAV = [
  { to: "dashboard", label: "Dashboard" },
  { to: "players", label: "Players" },
  { to: "teams", label: "Teams" },
  { to: "lines", label: "Lines" },
  { to: "arenas", label: "Arenas" },
  { to: "pending", label: "Pending Edits" },
];

export function AppShell() {
  const { project, dbPath } = useProject();
  if (!project || !dbPath) return <Navigate to="/" replace />;
  return <Shell project={project} dbPath={dbPath} />;
}

function Shell({ project, dbPath }: { project: ProjectSummaryDto; dbPath: string }) {
  const { update, close } = useProject();
  const navigate = useNavigate();
  const qc = useQueryClient();

  const save = useMutation({
    mutationFn: () => call("save_project", { projectId: project.project_id, outPath: dbPath }),
    onSuccess: async () => {
      const summary = await call("project_summary", { projectId: project.project_id });
      update(summary);
      qc.invalidateQueries();
    },
  });

  return (
    <div className={styles.shell}>
      <header className={styles.header}>
        <div className={styles.brand}>NHL Modding Studio</div>
        <div className={styles.meta}>
          {project.platform} · {project.role} · {project.source}
          {project.dirty && <span className={styles.dirty}>● unsaved</span>}
        </div>
        <div className={styles.actions}>
          <button onClick={() => save.mutate()} disabled={save.isPending || !project.dirty}>
            {save.isPending ? "Saving…" : "Save"}
          </button>
          <button
            onClick={() => {
              close();
              navigate("/", { replace: true });
            }}
          >
            Close
          </button>
        </div>
      </header>
      <div className={styles.body}>
        <nav className={styles.nav}>
          {NAV.map((n) => (
            <NavLink
              key={n.to}
              to={n.to}
              className={({ isActive }) => (isActive ? styles.navActive : styles.navLink)}
            >
              {n.label}
            </NavLink>
          ))}
        </nav>
        <main className={styles.content}>
          <Outlet />
        </main>
      </div>
      {save.isError && <div className={styles.error}>{String(save.error)}</div>}
    </div>
  );
}

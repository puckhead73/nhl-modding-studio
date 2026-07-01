import { NavLink, Navigate, Outlet, useNavigate } from "react-router-dom";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useProject } from "./lib/project";
import { call } from "./lib/client";
import { Pill } from "./components/primitives";
import headerLogo from "./assets/ui/NHL Modding Studio Header.png";
import type { ProjectSummaryDto } from "./bindings/ProjectSummaryDto";

const NAV = [
  { to: "dashboard", label: "Dashboard" },
  { to: "players", label: "Players" },
  { to: "teams", label: "Teams" },
  { to: "lines", label: "Lines" },
  { to: "arenas", label: "Arenas" },
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

  // A workspace with a roster attached saves both files in place; otherwise write the single db back.
  const save = useMutation({
    mutationFn: () =>
      project.roster_attached
        ? call("save_workspace", { projectId: project.project_id })
        : call("save_project", { projectId: project.project_id, outPath: dbPath }),
    onSuccess: async () => {
      update(await call("project_summary", { projectId: project.project_id }));
      qc.invalidateQueries();
    },
  });

  const navClass = ({ isActive }: { isActive: boolean }) =>
    "rounded-md px-3 py-1.5 text-sm font-medium transition-colors " +
    (isActive
      ? "bg-sky-900/40 text-sky-100"
      : "text-slate-300 hover:bg-slate-800/60 hover:text-slate-100");

  return (
    <div className="flex h-full flex-col bg-slate-950 text-slate-100">
      <header className="flex h-12 shrink-0 items-center gap-6 border-b border-slate-800 bg-slate-900/60 px-4">
        <img
          src={headerLogo}
          alt="NHL Modding Studio"
          draggable={false}
          className="h-7 w-auto shrink-0 select-none object-contain"
        />
        <nav className="flex flex-1 items-center gap-1">
          {NAV.map((n) => (
            <NavLink key={n.to} to={n.to} className={navClass}>
              {n.label}
            </NavLink>
          ))}
          <NavLink
            to="pending"
            className={({ isActive }) =>
              "ml-1 flex items-center gap-1.5 rounded-md border px-3 py-1.5 text-sm font-medium transition-colors " +
              (isActive
                ? "border-sky-700 bg-sky-900/40 text-sky-100"
                : project.dirty
                  ? "border-amber-800/70 bg-amber-900/30 text-amber-100 hover:bg-amber-900/50"
                  : "border-slate-700 text-slate-300 hover:bg-slate-800/60")
            }
          >
            Pending Edits
            {(project.undo_depth > 0 || project.dirty) && (
              <Pill tone={project.dirty ? "warn" : "neutral"}>{project.undo_depth}</Pill>
            )}
          </NavLink>
        </nav>
        <div className="flex items-center gap-2">
          <span className="text-xs text-slate-400">
            {project.platform} · {project.role} · {project.source}
          </span>
          {project.roster_attached &&
            (project.roster_aligned ? (
              <Pill tone="ok">Roster synced</Pill>
            ) : (
              <Pill tone="warn">Roster: nhlng-only</Pill>
            ))}
          <button
            type="button"
            onClick={() => save.mutate()}
            disabled={save.isPending || !project.dirty}
            title={project.roster_attached ? "Saves nhlng.db and the roster save together" : undefined}
            className="rounded-md border border-sky-500/40 bg-sky-600 px-3 py-1 text-xs font-medium text-white transition-colors hover:bg-sky-500 disabled:cursor-not-allowed disabled:opacity-50"
          >
            {save.isPending ? "Saving…" : "Save"}
          </button>
          <button
            type="button"
            onClick={() => {
              close();
              navigate("/", { replace: true });
            }}
            className="rounded-md border border-slate-700 px-2.5 py-1 text-xs text-slate-300 hover:bg-slate-800"
          >
            Close
          </button>
        </div>
      </header>
      <main className="flex min-h-0 flex-1 flex-col overflow-hidden">
        <div className="min-h-0 flex-1 overflow-y-auto px-6 py-6">
          {save.isError && (
            <p className="mb-3 rounded-md border border-rose-800 bg-rose-950/60 px-3 py-2 text-sm text-rose-100">
              {String(save.error)}
            </p>
          )}
          <Outlet />
        </div>
      </main>
    </div>
  );
}

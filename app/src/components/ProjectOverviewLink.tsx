import { NavLink } from "react-router-dom";
import { useProject } from "../lib/project";
import { useProjectOverview } from "../lib/projectOverview";
import { Pill } from "./primitives";

export function ProjectOverviewLink() {
  const { project } = useProject();
  const review = useProjectOverview();
  return <NavLink
    to="/app/database/pending"
    className={({ isActive }) =>
      "flex items-center gap-1.5 rounded-md border px-3 py-1 text-xs font-medium transition-colors " +
      (isActive
        ? "border-sky-700 bg-sky-900/40 text-sky-100"
        : project?.dirty
          ? "border-amber-800/70 bg-amber-900/30 text-amber-100 hover:bg-amber-900/50"
          : "border-slate-700 text-slate-300 hover:bg-slate-800/60")}
  >
    Project Overview
    {review.isSuccess && <Pill tone={project?.dirty ? "warn" : "neutral"}>{review.data.changes.length}</Pill>}
  </NavLink>;
}

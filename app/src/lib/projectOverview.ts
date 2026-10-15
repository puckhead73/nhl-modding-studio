import { useQuery } from "@tanstack/react-query";
import { call } from "./client";
import { useProject } from "./project";

/** One shared net-change query for the overview page and its navigation badge. */
export function useProjectOverview() {
  const { project, info } = useProject();
  return useQuery({
    queryKey: ["project-overview", project?.project_id, project?.workspace_revision,
      project?.undo_depth, project?.redo_depth, project?.dirty, info?.saves, info?.staged_files],
    queryFn: () => call("project_overview", { projectId: project!.project_id }),
    enabled: !!project && !!info,
  });
}

import { useProject } from "../lib/project";
import { LeagueImageryCard } from "../components/LeagueImageryCard";

/// P1-G — the title-wide league presentation imagery listing (a sibling of the per-league detail).
export function LeagueImageryPage() {
  const { project } = useProject();
  const projectId = project!.project_id;
  return (
    <div className="space-y-4">
      <LeagueImageryCard projectId={projectId} />
    </div>
  );
}

import { Link, useParams } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { FieldRow } from "../components/FieldRow";
import { Card, Pill, Spinner } from "../components/primitives";

export function TeamDetail() {
  const { id } = useParams<{ id: string }>();
  const { project } = useProject();
  const projectId = project!.project_id;
  const teamId = Number(id);

  const view = useQuery({
    queryKey: ["team", projectId, teamId],
    queryFn: () => call("team_view", { projectId, teamId }),
    enabled: Number.isFinite(teamId),
  });

  if (view.isLoading) return <Spinner label="Loading…" />;
  if (view.isError) return <p className="text-rose-400">{String(view.error)}</p>;
  const team = view.data;
  if (!team) return <p className="text-sm text-slate-400">Select a team.</p>;

  return (
    <div className="max-w-3xl space-y-3">
      <h2 className="flex items-center gap-2 text-lg font-semibold text-slate-100">
        Team #{team.team_id}
        <Pill tone="neutral">{team.roster_count} on roster</Pill>
      </h2>

      <Card title="Identity">
        {team.identity.map((f) => (
          <FieldRow key={f.studio_field} projectId={projectId} record={team.record} field={f} />
        ))}
      </Card>

      <Card title="Arena">
        {team.arena ? (
          <p className="text-sm text-slate-200">
            <Link className="text-sky-400 hover:underline" to={`/app/arenas/${team.arena.arena_id}`}>
              {team.arena.name || `Arena ${team.arena.arena_id}`}
            </Link>
            {team.arena.city && ` — ${team.arena.city}`}
          </p>
        ) : (
          <p className="text-xs text-slate-400">No arena resolved.</p>
        )}
      </Card>

      <details className="rounded-lg border border-slate-800 bg-slate-900/60 px-4 py-3">
        <summary className="cursor-pointer text-sm font-semibold text-slate-100">
          All team fields ({team.all_fields.length})
        </summary>
        <div className="mt-2">
          {team.all_fields.map((f) => (
            <FieldRow key={f.studio_field} projectId={projectId} record={team.record} field={f} />
          ))}
        </div>
      </details>
    </div>
  );
}

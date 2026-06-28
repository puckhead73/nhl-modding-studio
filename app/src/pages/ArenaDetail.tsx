import { useParams } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { FieldRow } from "../components/FieldRow";
import { Card, Spinner } from "../components/primitives";

export function ArenaDetail() {
  const { id } = useParams<{ id: string }>();
  const { project } = useProject();
  const projectId = project!.project_id;
  const arenaId = Number(id);

  const view = useQuery({
    queryKey: ["arena", projectId, arenaId],
    queryFn: () => call("arena_view", { projectId, arenaId }),
    enabled: Number.isFinite(arenaId),
  });

  if (view.isLoading) return <Spinner label="Loading…" />;
  if (view.isError) return <p className="text-rose-400">{String(view.error)}</p>;
  const arena = view.data;
  if (!arena) return <p className="text-sm text-slate-400">Select an arena.</p>;

  return (
    <div className="max-w-3xl space-y-3">
      <h2 className="text-lg font-semibold text-slate-100">Arena #{arena.arena_id}</h2>

      <Card title="Identity">
        {arena.identity.map((f) => (
          <FieldRow key={f.studio_field} projectId={projectId} record={arena.record} field={f} />
        ))}
      </Card>

      <details className="rounded-lg border border-slate-800 bg-slate-900/60 px-4 py-3">
        <summary className="cursor-pointer text-sm font-semibold text-slate-100">
          All arena fields ({arena.all_fields.length})
        </summary>
        <div className="mt-2">
          {arena.all_fields.map((f) => (
            <FieldRow key={f.studio_field} projectId={projectId} record={arena.record} field={f} />
          ))}
        </div>
      </details>
    </div>
  );
}

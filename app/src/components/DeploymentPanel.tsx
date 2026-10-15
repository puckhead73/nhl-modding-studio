// ONB-002 — the explicit, reviewed deployment of a project's saved outputs to the game
// installation and the save location: plan (read-only) → review every destination and its state
// → Deploy (the final user action) → records with backups → Restore. Cancel is non-mutating; a
// refusal leaves the project's work intact. Saving the project is a separate step and is never a
// deployment.

import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { Button, Card, Pill } from "./primitives";
import type { DeployPlanDto } from "../bindings/DeployPlanDto";
import type { DeployApplyDto } from "../bindings/DeployApplyDto";
import { formatBytes } from "./ExtractionPanel";

const STATE_LABEL: Record<string, string> = {
  "already-current": "already holds this file",
  unchanged: "unchanged since the project baseline / last deployment",
  "changed-externally": "changed outside Studio",
  absent: "not present yet (will be created)",
  unresolved: "no destination resolved",
  unreadable: "cannot be read",
};

export function DeploymentPanel() {
  const { project, info, updateInfo } = useProject();
  const qc = useQueryClient();
  const [plan, setPlan] = useState<DeployPlanDto | null>(null);
  const [allowChanged, setAllowChanged] = useState(false);
  const [applied, setApplied] = useState<DeployApplyDto | null>(null);
  const projectId = project!.project_id;

  const deployments = useQuery({
    queryKey: ["project_deployments", projectId],
    queryFn: () => call("project_deployments", { projectId }),
    enabled: !!info,
  });
  const refreshInfo = async () => {
    updateInfo(await call("project_info", { projectId }));
    await deployments.refetch();
  };
  const review = useMutation({
    mutationFn: () => call("project_deploy_plan", { projectId, allowChanged }),
    onSuccess: (p) => {
      setPlan(p);
      setApplied(null);
    },
  });
  const apply = useMutation({
    mutationFn: () => call("project_deploy_apply", { projectId, handle: plan!.handle }),
    onSuccess: async (r) => {
      setApplied(r);
      setPlan(null);
      await refreshInfo();
      qc.invalidateQueries();
    },
  });
  const restore = useMutation({
    mutationFn: (seq: number) => call("project_deploy_restore", { projectId, seq }),
    onSuccess: async () => {
      setPlan(null);
      await refreshInfo();
    },
  });

  if (!info) return null;
  const changing = plan?.items.filter((i) => i.state !== "already-current") ?? [];

  return (
    <Card
      title="Deploy to the game"
      subtitle="Copies the project's saved files to the game installation and the save location. Nothing is written until you press Deploy; every replaced file is backed up in the project first."
    >
      <div className="space-y-3 text-sm">
        <div className="text-xs text-slate-400">
          Game installation: <span className="font-mono text-slate-300">{info.destinations.picked_root}</span>
          {info.destinations.save_root && (
            <>
              {" "}· Save location: <span className="font-mono text-slate-300">{info.destinations.save_root}</span>
            </>
          )}
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Button variant="secondary" onClick={() => review.mutate()} disabled={review.isPending || apply.isPending}>
            {review.isPending ? "Reviewing…" : plan ? "Review again" : "Review deployment"}
          </Button>
          <label className="flex cursor-pointer items-center gap-2 text-xs text-slate-300">
            <input type="checkbox" checked={allowChanged} onChange={(e) => setAllowChanged(e.target.checked)} className="h-3 w-3" />
            Allow replacing files changed outside Studio
          </label>
          {plan && (
            <Button variant="secondary" onClick={() => setPlan(null)} disabled={apply.isPending}>
              Cancel
            </Button>
          )}
        </div>
        {review.isError && <p className="text-xs text-rose-300">{String(review.error)}</p>}

        {plan && (
          <div className="rounded-md border border-slate-800 bg-slate-950/40 p-3" data-testid="deploy-plan">
            {plan.reasons.length > 0 && (
              <ul className="mb-2 list-disc pl-4 text-xs text-amber-300">
                {plan.reasons.map((r, i) => (
                  <li key={i}>{r}</li>
                ))}
              </ul>
            )}
            <table className="w-full text-xs">
              <thead className="text-slate-400">
                <tr>
                  <th className="py-1 pr-2 text-left">File</th>
                  <th className="py-1 pr-2 text-left">Destination</th>
                  <th className="py-1 pr-2 text-left">State</th>
                </tr>
              </thead>
              <tbody>
                {plan.items.map((i) => (
                  <tr key={i.key} className="border-t border-slate-800 align-top">
                    <td className="py-1 pr-2 text-slate-200">
                      {i.label}
                      <div className="font-mono text-[11px] text-slate-500">{i.source.sha256.slice(0, 12)}… · {formatBytes(i.source.len)}</div>
                    </td>
                    <td className="py-1 pr-2 font-mono text-[11px] text-slate-400">{i.destination_path ?? "—"}</td>
                    <td className="py-1 pr-2">
                      <Pill tone={i.state === "already-current" ? "neutral" : i.blocked ? "warn" : "info"}>{STATE_LABEL[i.state] ?? i.state}</Pill>
                      {i.blocked && <div className="mt-1 text-amber-300">{i.blocked}</div>}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
            <div className="mt-2 text-xs text-slate-400">
              {changing.length} file{changing.length === 1 ? "" : "s"} would be written ({formatBytes(plan.replacement_bytes)}); {formatBytes(plan.backup_bytes)} of backups kept under the project.
            </div>
            <div className="mt-2 flex items-center gap-2">
              <Button onClick={() => apply.mutate()} disabled={!plan.deployable || apply.isPending}>
                {apply.isPending ? "Deploying…" : "Deploy"}
              </Button>
              {apply.isError && <span className="text-xs text-rose-300">{String(apply.error)}</span>}
            </div>
          </div>
        )}

        {applied && (
          <div className="rounded-md border border-emerald-600/40 bg-emerald-500/10 p-3 text-xs text-emerald-200" data-testid="deploy-result">
            Deployment {applied.seq} {applied.state}. {applied.destinations.length} destination{applied.destinations.length === 1 ? "" : "s"}; backups and the record are in <span className="font-mono">{applied.record_dir}</span>.
          </div>
        )}

        {deployments.data && deployments.data.length > 0 && (
          <div className="text-xs">
            <div className="mb-1 font-medium text-slate-200">Deployment history</div>
            <ul className="space-y-1">
              {deployments.data.map((d) => (
                <li key={d.seq} className="flex items-center gap-2 text-slate-300">
                  <span>#{d.seq}</span>
                  <span className="text-slate-500">{d.at}</span>
                  <Pill tone={d.state === "committed" ? "ok" : d.state === "restored" ? "neutral" : "warn"}>{d.state}</Pill>
                  {d.restorable && (
                    <Button variant="ghost" onClick={() => restore.mutate(d.seq)} disabled={restore.isPending}>
                      Restore originals
                    </Button>
                  )}
                </li>
              ))}
            </ul>
            {restore.isError && <p className="mt-1 text-rose-300">{String(restore.error)}</p>}
          </div>
        )}
      </div>
    </Card>
  );
}

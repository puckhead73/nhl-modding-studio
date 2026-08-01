// Team Outfits — the team-centric counterpart to the player-asset export. Where that page writes
// each player's head art named by the player, this writes each team's jersey, pants and socks named
// by the team (e.g. `detroit_red_wings_jersey_0_0_cm.png`). One run covers the whole roster; the
// backend resolves each team's id → name and matches the outfit textures under the install's art tree.

import { useMemo, useState } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import {
  Button,
  Card,
  EmptyState,
  ErrorBanner,
  PageScroll,
  Spinner,
  Stat,
} from "../components/primitives";
import { AssetGate } from "./PlayerAssets";
import type { OutfitExportSummaryDto } from "../bindings/OutfitExportSummaryDto";

type RunResult = { kind: "cancelled" } | { kind: "done"; summary: OutfitExportSummaryDto };

export function TeamOutfitsPage() {
  const { project, install } = useProject();
  const installDir = install?.install_dir ?? null;
  const [result, setResult] = useState<RunResult | null>(null);

  // Only the art roots are needed here — not the (heavy) per-player asset scan the browse page runs.
  const rootArgs = useMemo(
    () => ({
      install_dir: installDir ?? "",
      platform: install?.platform ?? undefined,
      rendering_override: null,
      fe_override: null,
    }),
    [installDir, install?.platform],
  );
  const roots = useQuery({
    queryKey: ["assetRoots", installDir],
    queryFn: () => call("asset_roots", { roots: rootArgs }),
    enabled: !!installDir,
  });

  const run = useMutation({
    mutationFn: async (): Promise<RunResult> => {
      const outDir = await openDialog({
        multiple: false,
        directory: true,
        title: "Choose a folder to export team outfits into",
      });
      if (typeof outDir !== "string") return { kind: "cancelled" };
      const summary = await call("export_team_outfits", {
        projectId: project!.project_id,
        roots: rootArgs,
        outDir,
      });
      return { kind: "done", summary };
    },
    onSuccess: setResult,
  });

  if (!project) {
    return <EmptyState title="No project open" hint="Open a project to export its team outfits." />;
  }
  const gate = AssetGate({ installDir, roots });
  if (gate) return gate;

  return (
    <PageScroll>
      <div className="max-w-2xl space-y-3 p-3">
        <Card
          title="Export team outfits"
          subtitle="Every team's jersey, pants and socks — colour, spec, normal and number/name maps — written out as PNGs and named by the team they belong to."
        >
          <div className="space-y-4 text-sm text-slate-300">
            <p className="text-slate-400">
              Files land under <code className="text-slate-300">jersey/</code>,{" "}
              <code className="text-slate-300">pant/</code> and{" "}
              <code className="text-slate-300">sock/</code> in the folder you choose, named by the
              team — e.g. <code className="text-slate-300">detroit_red_wings_jersey_0_0_cm.png</code>{" "}
              (older titles) or{" "}
              <code className="text-slate-300">detroit_red_wings_jersey_rbk_home_cm.png</code> (PS4,
              which keeps each jersey's variant name).
            </p>

            <div className="flex items-center gap-3 border-t border-slate-800 pt-3">
              <Button onClick={() => run.mutate()} disabled={run.isPending}>
                {run.isPending ? "Exporting…" : "Export all team outfits…"}
              </Button>
              {run.isPending ? (
                <Spinner label="Decoding every team's jerseys, pants and socks — this can take a little while…" />
              ) : null}
            </div>

            {run.error ? (
              <ErrorBanner title="Export failed" message={String(run.error)} />
            ) : null}
            {result?.kind === "cancelled" ? (
              <p className="text-xs text-slate-500">Export cancelled — no folder chosen.</p>
            ) : null}
          </div>
        </Card>

        {result?.kind === "done" ? <Summary summary={result.summary} /> : null}
      </div>
    </PageScroll>
  );
}

function Summary({ summary }: { summary: OutfitExportSummaryDto }) {
  const total = summary.jerseys_written + summary.pants_written + summary.socks_written;

  // Nothing written is the confusing case — say so loudly, and show where we looked.
  if (total === 0) {
    return (
      <Card title="Nothing was exported" subtitle={`${summary.teams_considered} teams resolved`}>
        <p className="text-sm text-amber-400">
          No outfit textures were found for this install.
        </p>
        <p className="mt-2 text-xs text-slate-400">
          Looked under: <code className="break-all text-slate-300">{summary.rendering_root}</code>
        </p>
        <p className="mt-2 text-xs text-slate-500">
          {summary.teams_considered === 0
            ? "No teams resolved from the database — the project may not have a team table loaded."
            : "Teams resolved, but no jersey/pant/sock textures were found there. The install's art may not be extracted, or this title stores outfits in a layout the exporter doesn't handle yet."}
        </p>
        {summary.failures.length > 0 ? <Failures failures={summary.failures} /> : null}
      </Card>
    );
  }

  return (
    <Card
      title="Last export"
      subtitle={`${summary.teams_considered} teams · ${formatBytes(summary.bytes_written)} written`}
    >
      <div className="grid grid-cols-3 gap-2">
        <Stat label="Jersey maps" value={summary.jerseys_written} />
        <Stat label="Pant maps" value={summary.pants_written} />
        <Stat label="Sock maps" value={summary.socks_written} />
      </div>
      {summary.failures.length > 0 ? (
        <Failures failures={summary.failures} />
      ) : (
        <p className="mt-3 text-xs text-slate-500">No failures.</p>
      )}
    </Card>
  );
}

function Failures({ failures }: { failures: string[] }) {
  return (
    <details className="mt-3 text-xs">
      <summary className="cursor-pointer select-none text-amber-500">
        {failures.length} skipped or failed
      </summary>
      <ul className="mt-1 max-h-56 space-y-0.5 overflow-y-auto text-slate-400">
        {failures.map((f, i) => (
          <li key={i}>{f}</li>
        ))}
      </ul>
    </details>
  );
}

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MB`;
  return `${(n / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

// One player's art: the decoded portrait plus what else ships for them.
//
// The route param is the `ubPc` record index — asset rows have no `wBIz` of their own, and the
// record index is what the export selects on.

import { useParams } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { Card, ErrorBanner, EmptyState, Pill, Spinner } from "../components/primitives";
import { HeadMeshViewer } from "../components/HeadMeshViewer";
import { usePlayerAssets } from "./PlayerAssets";
import type { PlayerAssetsDto } from "../bindings/PlayerAssetsDto";

export function PlayerAssetDetail() {
  const { id } = useParams();
  const { installDir, rootArgs, assets } = usePlayerAssets();
  const player = (assets.data ?? []).find((p) => String(p.bio_record) === id);

  if (!player) {
    return assets.isLoading ? (
      <Spinner label="Loading players…" />
    ) : (
      <EmptyState title="Player not found" hint="This record isn't in the current scan." />
    );
  }
  return <AssetDetail player={player} installDir={installDir} rootArgs={rootArgs} />;
}

function AssetDetail({
  player,
  installDir,
  rootArgs,
}: {
  player: PlayerAssetsDto;
  installDir: string | null;
  rootArgs: ReturnType<typeof usePlayerAssets>["rootArgs"];
}) {
  const preview = useQuery({
    queryKey: ["portraitPreview", player.portrait_path],
    queryFn: () => call("portrait_preview", { path: player.portrait_path!, crop: false }),
    enabled: !!player.portrait_path,
  });

  return (
    <div className="max-w-md space-y-3 p-3">
      <Card title={player.full_name || "—"} subtitle={player.slug ?? "no export name"}>
        <div className="space-y-3 text-sm">
          {player.portrait_path ? (
            preview.isLoading ? (
              <Spinner label="Decoding portrait…" />
            ) : preview.error ? (
              <ErrorBanner title="Couldn't decode this portrait" message={String(preview.error)} />
            ) : (
              // Checkerboard: portraits are transparent outside the subject, and EA pads them
              // onto a mostly-empty canvas — on a flat background that reads as "broken export".
              <img
                src={preview.data}
                alt={`${player.full_name} portrait`}
                className="w-full rounded border border-slate-800 bg-[repeating-conic-gradient(#1e293b_0_25%,#0f172a_0_50%)] bg-[length:16px_16px]"
              />
            )
          ) : (
            <p className="text-slate-500">No portrait ships for this player.</p>
          )}

          <div className="flex flex-wrap gap-1">
            {player.mesh_path ? (
              <Pill tone="ok">Own head model</Pill>
            ) : player.pool_head ? (
              <Pill tone="neutral">Shared pool head</Pill>
            ) : (
              <Pill tone="neutral">No head model</Pill>
            )}
            {player.texture_paths.length > 0 ? (
              <Pill tone="ok">
                {player.texture_paths.length === 1
                  ? "Head textures"
                  : `Head textures (${player.texture_paths.length})`}
              </Pill>
            ) : null}
            {player.slug ? null : <Pill tone="warn">Not exportable</Pill>}
          </div>

          {player.pool_head ? (
            <p className="text-xs text-slate-500">
              This player uses a generic head model shared with many others, so it isn't exported
              under their name.
            </p>
          ) : null}
          {player.slug ? null : (
            <p className="text-xs text-amber-500/80">
              Export names are built from the player's name and birthdate; this record is missing
              one, so their art can't be filed under a unique name.
            </p>
          )}
        </div>
      </Card>

      {player.mesh_path && installDir ? (
        <Card
          title="Head Mesh"
          subtitle="toggle submeshes to identify each part"
        >
          <HeadMeshViewer
            roots={{ ...rootArgs, install_dir: installDir }}
            headId={Number(player.head_id)}
            // PS4 heads are name-slug keyed, so a numeric id can't resolve the
            // bespoke `.mcd` — hand the viewer the paths already resolved in the
            // asset scan. RenderWare stays id-based (it merges NHL 09's separate
            // hair mesh, which isn't in the single `mesh_path`).
            meshPaths={
              rootArgs.platform === "ps4" ? [player.mesh_path] : undefined
            }
            texturePaths={
              rootArgs.platform === "ps4" ? player.texture_paths : undefined
            }
          />
        </Card>
      ) : null}

      {/* Raw EA ids and paths stay behind progressive disclosure — never the primary text. */}
      <details className="rounded-lg border border-slate-800 bg-slate-900/60 px-4 py-3 text-xs text-slate-500">
        <summary className="cursor-pointer select-none">Asset details</summary>
        <dl className="mt-2 space-y-1">
          <Detail label="Portrait id (ubPc.lOnr)" value={player.portrait_id} />
          <Detail label="Head id (ubPc.pPaD)" value={player.head_id} />
          <Detail label="Bio record" value={player.bio_record} />
          {player.portrait_path ? <Detail label="Portrait" value={player.portrait_path} /> : null}
          {player.mesh_path ? <Detail label="Mesh" value={player.mesh_path} /> : null}
          {player.texture_paths.map((t, i) => (
            <Detail key={t} label={i === 0 ? "Textures" : ""} value={t} />
          ))}
        </dl>
      </details>
    </div>
  );
}

function Detail({ label, value }: { label: string; value: string | number }) {
  return (
    <div className="flex gap-2">
      <dt className="w-40 shrink-0 text-slate-600">{label}</dt>
      <dd className="min-w-0 break-all font-mono text-slate-400">{value}</dd>
    </div>
  );
}

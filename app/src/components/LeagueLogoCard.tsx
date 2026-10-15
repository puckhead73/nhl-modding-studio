import { assetRoot } from "../lib/assetRoot";
import { useQuery } from "@tanstack/react-query";
import { NotExtractedPill } from "./GateHint";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { Button, Card, Pill } from "./primitives";
import { TextureThumbnail, useTextureReplace } from "./TextureReplace";
import { useArchiveMemberReplace } from "./ArchiveMemberReplace";

/// Phase 1.4 — a league's logo packages on this title: the family-wide art index the game maps
/// the league to (traced from the executable on Xbox 360, by reference on PS3, native on PS4), the
/// other leagues that share it, and each package family with a decoded thumbnail and a Replace
/// action through the shared texture surface (export → edit → reimport, backup written). Paths are
/// loose files under the install/extraction root; the disc's archive copies are a separate
/// placement question for tomorrow's cards.
export function LeagueLogoCard({ projectId, leagueCode }: { projectId: number; leagueCode: number }) {
  const { install, iso } = useProject();
  // The extraction/install root the texture surface addresses: the install dir, or the disc
  // staging folder (which holds only the database — logo files are then reported as absent).
  const installRoot = assetRoot(install) ?? iso?.root ?? null;
  const logo = useQuery({
    queryKey: ["league-logo", projectId, leagueCode, installRoot],
    queryFn: () => call("league_logo", { projectId, code: leagueCode, installRoot }),
  });
  const replace = useTextureReplace(installRoot, () => void logo.refetch());
  // PS4: the game reads the member from the packed archive, so the archive route is what reaches it.
  const archiveReplace = useArchiveMemberReplace(projectId, installRoot, () => void logo.refetch());
  if (logo.isLoading) return null;
  if (logo.isError) return (
    <Card title="League logo" actions={<Button variant="secondary" onClick={() => void logo.refetch()}>Retry</Button>}>
      <p role="alert" className="text-sm text-rose-300">{String(logo.error)}</p>
    </Card>
  );
  const l = logo.data ?? null;
  if (!l) return null;
  const tone = l.evidence === "native" || l.evidence === "traced" ? "ok" : "warn";
  return (
    <Card
      title="League logo"
      subtitle={`Logo art index ${String(l.art_index)} — one family-wide enum; the packages below are what the frontend loads for it.`}
    >
      <div className="mb-2 flex flex-wrap items-center gap-2 text-xs text-slate-400">
        <Pill tone={tone}>{`index ${l.evidence}`}</Pill>
        <span title={l.witness}>{l.witness.length > 90 ? l.witness.slice(0, 90) + "…" : l.witness}</span>
      </div>
      {l.shared_with.length > 0 && (
        <p className="mb-2 text-xs text-amber-300/90">
          Shared logo: replacing it also changes {l.shared_with.map(([c, n]) => `${n} (code ${String(c)})`).join(", ")}.
        </p>
      )}
      <table className="w-full text-left text-xs">
        <thead className="text-slate-500">
          <tr>
            <th className="py-1 pr-2">family</th>
            <th className="py-1 pr-2">file</th>
            <th className="py-1 pr-2">preview</th>
            <th className="py-1 pr-2"></th>
          </tr>
        </thead>
        <tbody>
          {l.variants.map((v) => (
            <tr key={v.rel_path} className="border-t border-slate-800">
              <td className="py-1 pr-2">{v.family}</td>
              <td className="py-1 pr-2 font-mono text-slate-400">
                {v.loose_rel_path ?? v.rel_path}
                {v.exists === false && <NotExtractedPill />}
                {v.exists == null && <Pill tone="neutral">no install root</Pill>}
              </td>
              <td className="py-1 pr-2">{v.exists && installRoot ? <TextureThumbnail installDir={installRoot} relPath={v.loose_rel_path ?? v.rel_path} className="h-12 w-12" maxEdge={64} /> : "—"}</td>
              <td className="py-1 pr-2">
                <span className="flex gap-1">
                  {v.exists && installRoot && (
                    <Button variant="secondary" onClick={() => void replace.pick(v.loose_rel_path ?? v.rel_path, `${v.family} l${String(l.art_index)}`)}>
                      Replace…
                    </Button>
                  )}
                  {v.archive && installRoot && (
                    <Button
                      variant="secondary"
                      title={`Replace the member inside ${v.archive} through the copied-target save (the copy the game reads)`}
                      onClick={() => void archiveReplace.pick(v.archive!, v.rel_path, `${v.family} l${String(l.art_index)}`)}
                    >
                      Replace in archive…
                    </Button>
                  )}
                </span>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {replace.dialog}
      {replace.result}
      {archiveReplace.dialog}
      {archiveReplace.result}
    </Card>
  );
}


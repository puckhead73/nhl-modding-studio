import { assetRoot } from "../lib/assetRoot";
import { useQuery } from "@tanstack/react-query";
import { NotExtractedPill } from "./GateHint";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { Button, Card, Pill } from "./primitives";
import { TextureThumbnail, useTextureReplace } from "./TextureReplace";
import { useArchiveMemberReplace } from "./ArchiveMemberReplace";

/// Phase 2.4 (C9, 20 Sep 2026) — a team's logo families on this title: every frontend package /
/// registry member keyed by the team's art id (the family table is measured per title on the
/// shipped archives), each with a decoded thumbnail, the loose Replace… (export → edit → reimport,
/// backup written) and, where the console reads the packed archive, Replace in archive… through
/// the copied-target save. Which screen draws which family remains a research question.
export function TeamLogoCard({ projectId, teamId }: { projectId: number; teamId: number }) {
  const { install, iso } = useProject();
  const installRoot = assetRoot(install) ?? iso?.root ?? null;
  const set = useQuery({
    queryKey: ["team-logo-set", projectId, teamId, installRoot],
    queryFn: () => call("team_logo_set", { projectId, teamId, installRoot }),
  });
  const replace = useTextureReplace(installRoot, () => void set.refetch());
  const archiveReplace = useArchiveMemberReplace(projectId, installRoot, () => void set.refetch());
  if (set.isLoading) return null;
  if (set.isError) return (
    <Card title="Team logos" actions={<Button variant="secondary" onClick={() => void set.refetch()}>Retry</Button>}>
      <p role="alert" className="text-sm text-rose-300">{String(set.error)}</p>
    </Card>
  );
  const l = set.data ?? null;
  if (!l) return null;
  const tag = `t${String(l.artid)}`;
  return (
    <Card
      title="Team logos"
      subtitle={`Art id ${String(l.artid)} — every logo family this title keys by it. The same id also names the rink logo, centre-ice logo and the jerseys' middle component (not listed here).`}
    >
      <div className="mb-2 flex flex-wrap items-center gap-2 text-xs text-slate-400" data-testid="team-logo-set">
        <Pill tone="ok">{`families ${l.evidence}`}</Pill>
        <span>{l.variants.length} families</span>
      </div>
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
            <tr key={v.rel_path} className="border-t border-slate-800" data-testid={`team-logo-${v.family}`}>
              <td className="py-1 pr-2">{v.family}</td>
              <td className="py-1 pr-2 font-mono text-slate-400">
                {v.loose_rel_path ?? v.rel_path}
                {v.exists === false && <NotExtractedPill />}
                {v.exists == null && <Pill tone="neutral">no install root</Pill>}
              </td>
              <td className="py-1 pr-2">
                {v.exists && installRoot ? (
                  <TextureThumbnail installDir={installRoot} relPath={v.loose_rel_path ?? v.rel_path} className="h-12 w-12" maxEdge={64} />
                ) : (
                  "—"
                )}
              </td>
              <td className="py-1 pr-2">
                <span className="flex gap-1">
                  {v.exists && installRoot && (
                    <Button variant="secondary" onClick={() => void replace.pick(v.loose_rel_path ?? v.rel_path, `${v.family} ${tag}`)}>
                      Replace…
                    </Button>
                  )}
                  {v.archive && installRoot && (
                    <Button
                      variant="secondary"
                      title={`Replace the member inside ${v.archive} through the copied-target save (the copy the game reads)`}
                      onClick={() => void archiveReplace.pick(v.archive!, v.rel_path, `${v.family} ${tag}`)}
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


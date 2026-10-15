import { assetRoot } from "../lib/assetRoot";
import { useQuery } from "@tanstack/react-query";
import { NotExtractedPill } from "./GateHint";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { Button, Card, Pill } from "./primitives";
import { TextureThumbnail, useTextureReplace } from "./TextureReplace";
import { useArchiveMemberReplace } from "./ArchiveMemberReplace";

/// P1-G — the title's league-keyed presentation imagery beyond the four league-logo families:
/// the playoff-tree conference/league logos, the championship and broadcast trophies and the
/// MUD-mode logos on Xbox 360 / PS3, and the league-shaped asset-path registry types on PS4 (large
/// logos, Memorial Cup, Champions tile, scouting region aliases, registered-but-unreferenced
/// icons). Each member previews and replaces (same dimensions) through the shared texture surface
/// on its loose/extracted file. The listing is per title: which id a given league maps to is
/// computed in game code (research 03) — the readings are the handoff's inferred labels for the
/// user to confirm on the morning cards.
export function LeagueImageryCard({ projectId }: { projectId: number }) {
  const { install, iso } = useProject();
  const installRoot = assetRoot(install) ?? iso?.root ?? null;
  const imagery = useQuery({
    queryKey: ["league-imagery", projectId, installRoot],
    queryFn: () => call("league_imagery", { projectId, installRoot }),
  });
  const replace = useTextureReplace(installRoot, () => void imagery.refetch());
  const archiveReplace = useArchiveMemberReplace(projectId, installRoot, () => void imagery.refetch());
  if (imagery.isLoading) return null;
  if (imagery.error) {
    return (
      <Card title="League presentation imagery">
        <p className="text-xs text-rose-300">{String(imagery.error)}</p>
      </Card>
    );
  }
  const li = imagery.data ?? null;
  if (!li) {
    return (
      <Card title="League presentation imagery" subtitle="No league-keyed imagery inventory for this title.">
        <p className="text-xs text-slate-500">The inventory covers Xbox 360 NHL 07–Legacy, PS3 NHL 08–Legacy and PS4 NHL 15–21.</p>
      </Card>
    );
  }
  return (
    <Card
      title="League presentation imagery"
      subtitle={`${li.families.length} league-keyed families on ${li.game_id} (${li.platform}) — binding read from ${li.registry}`}
    >
      {li.families.map((f) => (
        <section key={f.family} className="mb-4">
          <h3 className="text-sm font-semibold text-slate-100">
            {f.label} <span className="font-mono text-xs text-slate-500">{f.family}</span>
          </h3>
          {f.reading && <p className="text-xs text-slate-400">{f.reading}</p>}
          <p className="mt-0.5 text-xs text-slate-500">
            {f.consumers.length > 0 ? `Screens: ${f.consumers.join(", ")}` : "No screen references this family in UI text."}
          </p>
          <table className="mt-1 w-full text-left text-xs">
            <thead className="text-slate-500">
              <tr>
                <th className="py-1 pr-2">id</th>
                <th className="py-1 pr-2">file</th>
                <th className="py-1 pr-2">preview</th>
                <th className="py-1 pr-2"></th>
              </tr>
            </thead>
            <tbody>
              {f.members.map((m) => (
                <tr key={m.rel_path} className="border-t border-slate-800">
                  <td className="py-1 pr-2 tabular-nums text-slate-400">{String(m.id)}</td>
                  <td className="py-1 pr-2 font-mono text-slate-400">
                    {m.rel_path}
                    {m.exists === false && <NotExtractedPill />}
                    {m.exists == null && <Pill tone="neutral">no install root</Pill>}
                    {m.fr_rel_path && (
                      <span className="ml-1 text-slate-600" title={m.fr_rel_path}>
                        + French twin{m.fr_exists === false ? " (not extracted)" : ""}
                      </span>
                    )}
                    {f.texture_facts.find((t) => t.startsWith(`${String(m.id)}:`)) && (
                      <span className="ml-1 text-slate-600">{f.texture_facts.find((t) => t.startsWith(`${String(m.id)}:`))?.split(":").slice(1).join(" ")}</span>
                    )}
                  </td>
                  <td className="py-1 pr-2">
                    {m.exists && installRoot ? <TextureThumbnail installDir={installRoot} relPath={m.rel_path} className="h-12 w-12" maxEdge={64} /> : "—"}
                  </td>
                  <td className="py-1 pr-2">
                    <span className="flex flex-wrap gap-1">
                      {m.exists && installRoot && (
                        <Button variant="secondary" onClick={() => void replace.pick(m.rel_path, `${f.family} ${String(m.id)}`)}>
                          Replace…
                        </Button>
                      )}
                      {m.exists && installRoot && m.fr_rel_path && m.fr_exists && (
                        <Button variant="secondary" onClick={() => void replace.pick(m.fr_rel_path!, `${f.family} ${String(m.id)} (French)`)}>
                          Replace French…
                        </Button>
                      )}
                      {m.archive && installRoot && (
                        <Button
                          variant="secondary"
                          title={`Replace the member inside ${m.archive} through the copied-target save (the copy the game reads)`}
                          onClick={() => void archiveReplace.pick(m.archive!, m.rel_path, `${f.family} ${String(m.id)}`)}
                        >
                          Replace in archive…
                        </Button>
                      )}
                      {m.archive && installRoot && m.fr_rel_path && (
                        <Button variant="secondary" onClick={() => void archiveReplace.pick(m.archive!, m.fr_rel_path!, `${f.family} ${String(m.id)} (French)`)}>
                          French in archive…
                        </Button>
                      )}
                    </span>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      ))}
      <ul className="mt-2 list-disc space-y-1 pl-5 text-xs text-slate-500">
        {li.notes.map((n, i) => (
          <li key={i}>{n}</li>
        ))}
      </ul>
      {replace.dialog}
      {replace.result}
      {archiveReplace.dialog}
      {archiveReplace.result}
    </Card>
  );
}

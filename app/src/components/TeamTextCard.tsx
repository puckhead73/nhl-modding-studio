import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { Card, Pill } from "./primitives";
import { DictionaryRenamePanel } from "./DictionaryRenamePanel";

/// Phase 2.3 — the text a team owns in the title's dictionary (name, city, abbreviation, two-line
/// name, nicknames, ticker), keyed by the team row's `artabbr`, shown beside the database's own
/// name strings. Each key edits through the shared dictionary panel (another key's text, or free
/// text re-encoded in place when it fits). Which screen reads which of these — dictionary keys or
/// database strings — is a runtime question; the card says so rather than guessing.
export function TeamTextCard({ projectId, teamId, dictionarySha256 }: { projectId: number; teamId: number; dictionarySha256: string | null }) {
  const q = useQuery({
    queryKey: ["team-text", projectId, teamId, dictionarySha256],
    queryFn: () => call("team_text", { projectId, teamId }),
  });
  const [editing, setEditing] = useState<string | null>(null);
  if (q.isLoading) return null;
  const t = q.data ?? null;
  if (!t) return null;
  return (
    <Card
      title="Team text in the dictionary"
      subtitle={t.artabbr ? `Keys derived from artabbr "${t.artabbr}"; the database row itself carries "${t.db_fullname ?? ""}" / "${t.db_shortname ?? ""}" / "${t.db_abbrname ?? ""}".` : "This team row has no artabbr; no per-team dictionary keys can be derived."}
    >
      {t.keys.length > 0 && (
        <table className="w-full text-left text-xs">
          <thead className="text-slate-500">
            <tr>
              <th className="py-1 pr-2">family</th>
              <th className="py-1 pr-2">key</th>
              <th className="py-1 pr-2">text</th>
              <th className="py-1 pr-2"></th>
            </tr>
          </thead>
          <tbody>
            {t.keys.map((k) => (
              <tr key={k.key} className="border-t border-slate-800">
                <td className="py-1 pr-2" title={k.note}>
                  {k.family}
                </td>
                <td className="py-1 pr-2 font-mono text-slate-400">
                  {k.row_key ?? k.key}
                  {k.lookup === "by-hash" && <Pill tone="neutral">by hash</Pill>}
                  {k.lookup === "absent" && <Pill tone="neutral">{dictionarySha256 ? "not in this dictionary" : "no dictionary attached"}</Pill>}
                </td>
                <td className="py-1 pr-2 text-slate-200">{k.text ?? "—"}</td>
                <td className="py-1 pr-2">
                  {k.row_key && dictionarySha256 && (
                    <button
                      type="button"
                      className="text-[11px] text-sky-400 underline-offset-2 hover:underline"
                      onClick={() => setEditing(editing === k.row_key ? null : k.row_key!)}
                    >
                      {editing === k.row_key ? "close" : "edit…"}
                    </button>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
      {editing && dictionarySha256 && (
        <div className="mt-3">
          <DictionaryRenamePanel key={`${projectId}:${teamId}:${editing}`} projectId={projectId} initialKey={editing} keyEvidence={null} dictionarySha256={dictionarySha256} />
        </div>
      )}
      <ul className="mt-2 list-disc space-y-1 pl-5 text-xs text-slate-500">
        {t.notes.map((n, i) => (
          <li key={i}>{n}</li>
        ))}
      </ul>
    </Card>
  );
}

import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { call, type PlayerAssignment } from "../lib/client";
import { queueFieldWrite } from "../lib/fieldWrites";
import { refreshFields } from "../lib/useFieldEdit";
import { useProject } from "../lib/project";

export function AssignmentRow({ item, onChange }: { item: PlayerAssignment; onChange: (choice: string) => Promise<void> }) {
  const [pending, setPending] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const selected = pending ?? item.current;
  const users = item.options.find(option => option.key === selected)?.users ?? 0;
  const visible = item.options.filter(option => option.key === selected || option.label.toLocaleLowerCase().includes(search.trim().toLocaleLowerCase()));
  return <div className="space-y-1 border-b border-slate-800 py-3">
    {item.options.length > 30 && <input aria-label={`Search ${item.label}`} placeholder="Find a player or option…"
      className="w-full rounded border border-slate-700 bg-slate-950 p-2 text-sm text-slate-100" value={search} onChange={event => setSearch(event.target.value)} />}
    <label className="flex flex-wrap items-center gap-3 text-sm">
      <span className="w-40 text-slate-200">{item.label}</span>
      <select aria-label={item.label} value={selected} disabled={pending != null}
        className="min-w-0 flex-1 rounded border border-slate-700 bg-slate-950 p-2 text-slate-100"
        onChange={event => {
          const value = event.target.value;
          setPending(value); setError(null);
          void onChange(value).catch(reason => setError(String(reason))).finally(() => setPending(null));
        }}>
        {visible.map(option => <option key={option.key} value={option.key} disabled={option.users === 0}>{option.label}</option>)}
      </select>
    </label>
    <p className="text-xs text-slate-500">{pending ? "Saving assignment…" : item.availability}</p>
    {users > 1 && <p className="text-xs text-slate-500">Used by {users} players in the original database.</p>}
    {error && <p role="alert" className="text-sm text-rose-300">{error}</p>}
  </div>;
}

export function Nhl07PlayerAssignments({ projectId, playerId }: { projectId: number; playerId: number }) {
  const qc = useQueryClient();
  const { update } = useProject();
  const key = ["player-assignments", projectId, playerId];
  const query = useQuery({ queryKey: key, queryFn: () => call("player_assignments", { projectId, playerId }) });
  if (query.error) return <p role="alert" className="text-rose-300">{String(query.error)}</p>;
  if (!query.data) return <p className="text-slate-400">Loading player assignments…</p>;
  return <div className="max-w-4xl space-y-5">
    <p className="text-sm text-slate-400">Choose an existing setup using the players who originally used it. Only this player's assignments change; shared asset files stay intact.</p>
    {["Equipment", "Appearance", "Portrait and commentary"].map(section => <section key={section} className="rounded-lg border border-slate-800 bg-slate-900/40 p-4">
      <h3 className="font-semibold text-slate-100">{section}</h3>
      {section === "Appearance" && <p className="mt-2 text-xs text-slate-400">Head selections include skin tone and eye colors, which you can then adjust. Options use original player setups; their appearance needs in-game review.</p>}
      {section === "Portrait and commentary" && <p className="mt-2 text-xs text-slate-400">Commentary selections keep the player's two audio references together. In-game playback still needs verification.</p>}
      {query.data!.filter(item => item.section === section).map(item => <AssignmentRow key={`${projectId}/${playerId}/${item.key}`} item={item} onChange={async choice => {
        await queueFieldWrite(projectId, () => call("set_player_assignment", { projectId, playerId, assignment: item.key, choice }));
        // Publish the confirmed selection before releasing its immediate local echo.
        qc.setQueryData<PlayerAssignment[]>(key, prior => prior?.map(row => row.key === item.key ? { ...row, current: choice, availability: "Checking game source…" } : row));
        void refreshFields(qc, projectId, update).catch(() => {});
      }} />)}
    </section>)}
  </div>;
}

import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { queueFieldWrite } from "../lib/fieldWrites";
import { refreshFields } from "../lib/useFieldEdit";
import { Button, Card } from "./primitives";

export function Nhl07MembershipCard({projectId,teamId}:{projectId:number;teamId:number}) {
  const qc=useQueryClient(); const {update}=useProject();
  const options=useQuery({queryKey:["membership",projectId,teamId],queryFn:()=>call("team_membership_options",{projectId,teamId})});
  const teams=useQuery({queryKey:["teams",projectId],queryFn:()=>call("list_teams",{projectId})});
  const [league,setLeague]=useState<number|null>(null),[conf,setConf]=useState<number|null>(null),[division,setDivision]=useState<number|null>(null);
  const [swap,setSwap]=useState("");const [busy,setBusy]=useState(false),[error,setError]=useState<string|null>(null);
  const o=options.data, current=o?.current;
  const code=league??Number(current?.league_code), c=conf??Number(current?.conference?.value), d=division??Number(current?.division?.value);
  const groups=o?.groups_by_league.find(g=>Number(g.league_code)===code);
  const changingLeague=!!current && code!==Number(current.league_code);
  async function save(){
    setBusy(true);setError(null);
    try {
      await queueFieldWrite<unknown>(projectId,()=> changingLeague && swap ? call("swap_team_memberships",{projectId,first:teamId,second:Number(swap)}) : call("set_team_membership",{projectId,teamId,leagueCode:code,conference:c,division:d}));
      setLeague(null);setConf(null);setDivision(null);setSwap("");
      await refreshFields(qc,projectId,update);
    }catch(e){setError(String(e));}finally{setBusy(false);}
  }
  if(options.error)return <p role="alert">{String(options.error)}</p>;
  if(!o)return null;
  if(teamId===30 || teamId===31)return <Card title="League and division"><p className="text-sm text-slate-400">All-Star teams retain their original organization.</p></Card>;
  const input="mt-1 block w-full rounded border border-slate-700 bg-slate-950 p-2 text-sm";
  return <Card title="League and division">
    <fieldset disabled={busy} className="grid gap-3 sm:grid-cols-3">
      <label className="text-sm">League<select aria-label="League" className={input} value={code} onChange={e=>{const n=Number(e.target.value);setLeague(n);setSwap("");const first=o.groups_by_league.find(g=>Number(g.league_code)===n)?.pairs[0];setConf(first?Number(first[0]):null);setDivision(first?Number(first[1]):null);}}>{o.leagues.map(l=><option key={String(l.value)} value={Number(l.value)}>{l.label}</option>)}</select></label>
      {!changingLeague&&<><label className="text-sm">Conference<select aria-label="Conference" className={input} value={c} onChange={e=>{const n=Number(e.target.value);setConf(n);setDivision(Number(groups?.pairs.find(p=>Number(p[0])===n)?.[1]));}}>{groups?.conferences.map(v=><option key={String(v.value)} value={Number(v.value)}>{v.label}</option>)}</select></label>
      <label className="text-sm">Division<select aria-label="Division" className={input} value={d} onChange={e=>setDivision(Number(e.target.value))}>{groups?.divisions.filter(v=>groups.pairs.some(p=>Number(p[0])===c&&Number(p[1])===Number(v.value))).map(v=><option key={String(v.value)} value={Number(v.value)}>{v.label}</option>)}</select></label></>}
      {changingLeague&&<label className="text-sm sm:col-span-2">Swap with<select aria-label="Swap with team" className={input} value={swap} onChange={e=>setSwap(e.target.value)}><option value="">Select a team</option>{teams.data?.filter(t=>Number(t.league)===code&&t.team_id!==teamId).map(t=><option key={t.team_id} value={t.team_id}>{t.name}</option>)}</select></label>}
    </fieldset>
    {changingLeague&&<p className="mt-2 text-xs text-slate-400">Both teams exchange their league, conference, division, and place in the schedule.</p>}
    <Button className="mt-3" disabled={busy||(changingLeague&&!swap)} onClick={()=>void save()}>{busy?"Saving…":changingLeague?"Swap teams":"Save organization"}</Button>
    {error&&<p role="alert" className="mt-2 text-sm text-rose-300">{error}</p>}
  </Card>;
}

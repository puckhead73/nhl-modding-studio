import { useState } from "react";
import { useQueries, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "react-router-dom";
import { call } from "../lib/client";
import { Card, Button } from "./primitives";
import { useProject } from "../lib/project";
import { queueFieldWrite } from "../lib/fieldWrites";
import { refreshFields } from "../lib/useFieldEdit";
import type { TeamSummaryDto } from "../bindings/TeamSummaryDto";
import type { MembershipOptionsDto } from "../bindings/MembershipOptionsDto";

export function Nhl07LeagueStructure({projectId,members}:{projectId:number;members:TeamSummaryDto[]}) {
  const queries=useQueries({queries:members.map(t=>({queryKey:["membership",projectId,t.team_id],queryFn:()=>call("team_membership_options",{projectId,teamId:t.team_id})}))});
  const teams=useQuery({queryKey:["teams",projectId],queryFn:()=>call("list_teams",{projectId})});
  if(queries.some(q=>q.isLoading)||teams.isLoading)return <Card title="Competition structure">Loading teams…</Card>;
  if(queries.some(q=>q.error)||teams.error)return <Card title="Competition structure"><p role="alert">Unable to load team organization.</p></Card>;
  const rows=members.map((team,i)=>({team,options:queries[i].data})).filter(r=>r.options!=null);
  rows.sort((a,b)=>Number(a.options!.current.conference?.value)-Number(b.options!.current.conference?.value)||Number(a.options!.current.division?.value)-Number(b.options!.current.division?.value)||a.team.name.localeCompare(b.team.name));
  return <Card title="Competition structure">
    <div className="overflow-x-auto"><table className="w-full text-left text-sm"><thead className="text-slate-400"><tr>{["Team","League","Conference","Division",""].map((s,i)=><th key={i} className="p-2">{s}</th>)}</tr></thead>
      <tbody>{rows.map(({team,options})=><OrganizationRow key={projectId+"/"+team.team_id} projectId={projectId} team={team} options={options!} teams={teams.data??[]}/>)}</tbody>
    </table></div>
    <p className="mt-3 text-xs text-slate-400">League changes exchange two teams and their schedule places. Adding or removing leagues/conferences and moving whole divisions require further NHL 07 mapping.</p>
  </Card>;
}
function OrganizationRow({projectId,team,options:o,teams}:{projectId:number;team:TeamSummaryDto;options:MembershipOptionsDto;teams:TeamSummaryDto[]}) {
  const qc=useQueryClient();const {update}=useProject();
  const [draft,setDraft]=useState<{league:number;conference:number;division:number}|null>(null);
  const [swap,setSwap]=useState("");const [busy,setBusy]=useState(false);const [error,setError]=useState<string|null>(null);
  const current={league:Number(o.current.league_code),conference:Number(o.current.conference?.value??0),division:Number(o.current.division?.value??0)};
  const value=draft??current;const groups=o.groups_by_league.find(g=>Number(g.league_code)===value.league);
  const changing=value.league!==current.league;const special=team.team_id===30||team.team_id===31;
  const style="w-full rounded border border-slate-700 bg-slate-950 p-2 text-xs disabled:opacity-50";
  async function save(){setBusy(true);setError(null);try{
    await queueFieldWrite<unknown>(projectId,()=>changing?call("swap_team_memberships",{projectId,first:team.team_id,second:Number(swap)}):call("set_team_membership",{projectId,teamId:team.team_id,leagueCode:value.league,conference:value.conference,division:value.division}));
    await refreshFields(qc,projectId,update);setDraft(null);setSwap("");
  }catch(e){setError(String(e));}finally{setBusy(false);}}
  return <tr className="border-t border-slate-800 align-top">
    <td className="p-2"><Link className="text-sky-300" to={"/app/database/teams/"+team.team_id}>{team.name}</Link>{error&&<p role="alert" className="mt-2 text-xs text-rose-300">{error}</p>}</td>
    <td className="p-2"><select aria-label={team.name+" league"} className={style} disabled={busy||special||!!o.blocker} value={value.league} onChange={e=>{const league=Number(e.target.value);const pair=o.groups_by_league.find(g=>Number(g.league_code)===league)?.pairs[0];setDraft({league,conference:Number(pair?.[0]??0),division:Number(pair?.[1]??0)});setSwap("");}}>{o.leagues.map(l=><option key={String(l.value)} value={Number(l.value)}>{l.label}</option>)}</select></td>
    {changing?<td colSpan={2} className="p-2"><select aria-label={team.name+" swap with"} className={style} value={swap} disabled={busy} onChange={e=>setSwap(e.target.value)}><option value="">Choose a team to exchange</option>{teams.filter(t=>Number(t.league)===value.league&&t.team_id!==30&&t.team_id!==31).map(t=><option key={t.team_id} value={t.team_id}>{t.name}</option>)}</select></td>:special?<td colSpan={2} className="p-2 text-slate-400">All-Star teams</td>:<>
    <td className="p-2"><select aria-label={team.name+" conference"} className={style} disabled={busy||!!o.blocker} value={value.conference} onChange={e=>{const conference=Number(e.target.value);const division=Number(groups?.pairs.find(p=>Number(p[0])===conference)?.[1]??0);setDraft({...value,conference,division});}}>{groups?.conferences.map(c=><option key={String(c.value)} value={Number(c.value)}>{c.label}</option>)}</select></td>
    <td className="p-2"><select aria-label={team.name+" division"} className={style} disabled={busy||!!o.blocker} value={value.division} onChange={e=>setDraft({...value,division:Number(e.target.value)})}>{groups?.divisions.filter(d=>groups.pairs.some(p=>Number(p[0])===value.conference&&Number(p[1])===Number(d.value))).map(d=><option key={String(d.value)} value={Number(d.value)}>{d.label}</option>)}</select></td></>}
    <td className="p-2">{!special&&<div className="flex gap-2"><Button disabled={busy||!draft||!!o.blocker||(changing&&!swap)} onClick={()=>void save()}>{busy?"Saving…":changing?"Swap":"Apply"}</Button>{draft&&<Button variant="secondary" disabled={busy} onClick={()=>{setDraft(null);setSwap("");setError(null);}}>Cancel</Button>}</div>}</td>
  </tr>;
}

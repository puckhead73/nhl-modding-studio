import { useEffect, useRef, useState } from "react";
import { Link } from "react-router-dom";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useAssetRootArgs, useProject } from "../lib/project";
import { queueFieldWrite } from "../lib/fieldWrites";
import { refreshFields } from "../lib/useFieldEdit";
import { Button } from "./primitives";
import type { ScheduleViewDto } from "../bindings/ScheduleViewDto";
import type { ScheduledGameDto } from "../bindings/ScheduledGameDto";

export function flexibleSchedule(name:string) {return ["nhlschedulelongleap","nhlschedulelongnoleap","nhlschedulemedium","nhlschedulesmall","intldelschedule","intlelitserienschedule","intlsmliigaschedule"].includes(name);}
export function scheduleName(name: string) {
  return ({ nhlschedulelongleap: "NHL · 82 games · leap season", nhlschedulelongnoleap: "NHL · 82 games",
    nhlschedulemedium: "NHL · 58 games", nhlschedulesmall: "NHL · 29 games",
    intldelschedule: "DEL season", intlelitserienschedule: "Elitserien season", intlsmliigaschedule: "SM-liiga season" } as Record<string,string>)[name] ?? name;
}
export function scheduleRows(view: ScheduleViewDto) {
  return view.games.map(g => ({index:Number(g.index), month:Number(g.month), day:Number(g.day), home_slot:Number(g.home_slot), away_slot:Number(g.away_slot), round:Number(g.round), status:g.status == null ? null : Number(g.status)}));
}
function OpponentLogo({ projectId, teamId, name }: { projectId:number; teamId:number; name:string }) {
  const { installDir, rootArgs } = useAssetRootArgs();
  const logo=useQuery({queryKey:["teamLogo",projectId,installDir,teamId],queryFn:()=>call("team_logo",{projectId,roots:rootArgs,teamId}),enabled:!!installDir,staleTime:Infinity});
  const [failed,setFailed]=useState(false);
  return logo.data && !failed ? <img src={logo.data} alt="" title={name} className="h-8 w-8 shrink-0 object-contain" onError={()=>setFailed(true)} /> : null;
}

export function TeamScheduleCalendar({projectId,teamId,league}:{projectId:number;teamId:number;league:number|null}) {
  const tables=useQuery({queryKey:["schedule-tables",projectId],queryFn:()=>call("schedule_tables",{projectId})});
  const choices=(tables.data??[]).filter(t=>t.league_code!=null && Number(t.league_code)===league);
  const [picked,setPicked]=useState("");
  const table=choices.find(t=>t.studio_table===picked)??choices.find(t=>t.name==="nhlschedulelongnoleap")??choices[0];
  if(tables.error)return <p role="alert">{String(tables.error)}</p>;
  if(tables.isLoading)return <p>Loading schedules…</p>;
  if(!table)return <p className="text-sm text-slate-400">This competition has no editable source schedule in NHL 07.</p>;
  return <div className="space-y-4">
    <div className="flex flex-wrap items-center justify-between gap-3">
      <label className="text-sm">Schedule <select aria-label="Schedule variant" value={table.studio_table} onChange={e=>setPicked(e.target.value)} className="ml-2 rounded border border-slate-700 bg-slate-950 p-2">
        {choices.map(t=><option key={t.studio_table} value={t.studio_table}>{scheduleName(t.name)}</option>)}
      </select></label>
      <Link to={`/app/database/leagues/${league}`} className="text-sm text-sky-300">League schedule · Import / Export CSV</Link>
    </div>
    <Calendar key={`${projectId}/${teamId}/${table.studio_table}`} projectId={projectId} teamId={teamId} table={table.studio_table} leap={table.name.endsWith("longleap")} />
  </div>;
}

function Calendar({projectId,teamId,table,leap}:{projectId:number;teamId:number;table:string;leap:boolean}) {
  const qc=useQueryClient(); const {update}=useProject(); const key=["schedule-view",projectId,table];
  const query=useQuery({queryKey:key,queryFn:()=>call("schedule_view",{projectId,studioTable:table})});
  const [season,setSeason]=useState(leap?2007:2006);
  const [month,setMonth]=useState(9);
  const [day,setDay]=useState<number|null>(null);
  const [snapshot,setSnapshot]=useState<ScheduleViewDto|null>(null);
  const [editing,setEditing]=useState<number|null>(null);
  const [editTeam,setEditTeam]=useState(teamId);
  const [opponent,setOpponent]=useState(""); const [home,setHome]=useState(true);
  const [date,setDate]=useState(""); const [busy,setBusy]=useState(false); const [error,setError]=useState<string|null>(null);
  const dialog=useRef<HTMLDialogElement>(null);
  useEffect(()=>{if(day!=null)dialog.current?.showModal();else dialog.current?.close();},[day]);
  const initialized=useRef(false);
  useEffect(()=>{if(query.data && !initialized.current){ initialized.current=true;const own=query.data.slot_map.find(s=>Number(s.team_id)===teamId)?.slot;
    const first=query.data.games.find(g=>Number(g.home_slot)===Number(own)||Number(g.away_slot)===Number(own));if(first)setMonth(Number(first.month));}},[query.data,teamId]);
  const view=query.data; const year=month>=8?season:season+1;
  const own=Number(view?.slot_map.find(s=>Number(s.team_id)===teamId)?.slot??-1);
  const myGames=(view?.games??[]).filter(g=>Number(g.home_slot)===own||Number(g.away_slot)===own);
  // Resolve NHL07's stock All-Star identities through the current slot map.
  const isAllStar=(g:ScheduledGameDto)=>[g.home_slot,g.away_slot].every(slot=>[30,31].includes(Number(view?.slot_map.find(t=>Number(t.slot)===Number(slot))?.team_id)));
  const calendarGames=(view?.games??[]).filter(g=>Number(g.home_slot)===own||Number(g.away_slot)===own||isAllStar(g));
  const editOwn=Number(snapshot?.slot_map.find(t=>Number(t.team_id)===editTeam)?.slot??own);
  const days=new Date(year,month+1,0).getDate(), offset=new Date(year,month,1).getDay();
  const dateText=(d:number)=>`${year}-${String(month+1).padStart(2,"0")}-${String(d).padStart(2,"0")}`;
  function choose(g?:ScheduledGameDto,source=snapshot,selectedDay=day){
    const owner=g&&isAllStar(g)&&Number(g.home_slot)!==own&&Number(g.away_slot)!==own
      ?Number(source?.slot_map.find(t=>Number(t.slot)===Number(g.home_slot))?.team_id):teamId;
    const ownerSlot=Number(source?.slot_map.find(t=>Number(t.team_id)===owner)?.slot??own);
    setEditTeam(owner);setEditing(g?.record??null);setHome(g?Number(g.home_slot)===ownerSlot:true);
    const other=g?source?.slot_map.find(t=>Number(t.slot)===Number(Number(g.home_slot)===ownerSlot?g.away_slot:g.home_slot)):null;
    setOpponent(other?String(other.team_id):"");setDate(g?`${Number(g.month)>=8?season:season+1}-${String(Number(g.month)+1).padStart(2,"0")}-${String(Number(g.day)+1).padStart(2,"0")}`:dateText(selectedDay!));setError(null);
  }
  function openDay(d:number,g?:ScheduledGameDto){setSnapshot(view!);setDay(d);choose(g,view!,d);}
  async function apply(remove:boolean){
    if(!snapshot||busy)return;setBusy(true);setError(null);
    const parts=date.split("-").map(Number); const before=qc.getQueryData<ScheduleViewDto>(key);
    try{
      const [y,m,d]=parts; if(!remove && (!opponent||!y||new Date(y,m-1,d).getMonth()!==m-1||new Date(y,m-1,d).getDate()!==d))throw new Error("Choose an opponent and a valid date.");
      if(!remove && y !== (m>=9?season:season+1)) throw new Error("Choose a date within the displayed season.");
      // Echo the shared fixture immediately; the validated result replaces this provisional row.
      const other=snapshot.slot_map.find(s=>Number(s.team_id)===Number(opponent));
      if(remove||other){let games=snapshot.games.filter(g=>g.record!==editing);
        if(!remove){const prior=snapshot.games.find(g=>g.record===editing);games=[...games,{record:editing??games.length,index:BigInt(editing??games.length),month:BigInt(m-1),day:BigInt(d-1),home_slot:BigInt(home?editOwn:Number(other!.slot)),away_slot:BigInt(home?Number(other!.slot):editOwn),home:null,away:null,date:"",round:prior?.round??0n,status:prior?.status??0n,round_label:null,home_label:null,away_label:null}];}
        qc.setQueryData(key,{...snapshot,games});}
      const result=await queueFieldWrite(projectId,()=>call("team_schedule_edit",{projectId,studioTable:table,team:editTeam,fingerprint:snapshot.fingerprint,record:editing,remove,opponent:opponent?Number(opponent):null,home,month:parts[1]||1,day:parts[2]||1,expectedGames:scheduleRows(snapshot)}));
      qc.setQueryData(key,result);setDay(null);void refreshFields(qc,projectId,update).catch(e=>setError(String(e)));
    }catch(e){if(before)qc.setQueryData(key,before);setError(String(e));}finally{setBusy(false);}
  }
  if(query.error)return <p role="alert">{String(query.error)}</p>;
  if(!view)return <p>Loading calendar…</p>;
  return <div className="space-y-3">
    <div className="flex flex-wrap items-center justify-between gap-3">
      <div className="flex items-center gap-3"><Button variant="secondary" aria-label="Previous month" onClick={()=>setMonth(m=>(m+11)%12)}>‹</Button><h3 className="min-w-40 text-center font-semibold">{new Date(year,month,1).toLocaleDateString(undefined,{month:"long",year:"numeric"})}</h3><Button variant="secondary" aria-label="Next month" onClick={()=>setMonth(m=>(m+1)%12)}>›</Button></div>
      <label className="text-xs text-slate-400">Season starts <input aria-label="Calendar season start year" type="number" min={1900} max={2199} value={season} onChange={e=>{const v=Number(e.target.value);if(v>=1900&&v<=2199){if(leap && new Date(v+1,1,29).getMonth()!==1){setError("The leap-season template needs a season ending in a leap year, such as 2007–2008.");}else{setSeason(v);setError(null);}}}} className="ml-2 w-20 rounded border border-slate-700 bg-slate-950 p-1" /></label>
    </div>
    <p className="text-xs text-slate-400">{myGames.length} games · Home — vs · Away — @. Click a day to add or change a game. The year controls the calendar display; NHL 07 stores only month and day.</p>
    {error&&day==null&&<p role="alert" className="text-rose-300">{error}</p>}
    <div className="overflow-x-auto"><div className="grid min-w-[650px] grid-cols-7 gap-px overflow-hidden rounded border border-slate-700 bg-slate-700">
      {["Sun","Mon","Tue","Wed","Thu","Fri","Sat"].map(d=><div key={d} className="bg-slate-900 p-2 text-center text-xs text-slate-400">{d}</div>)}
      {Array.from({length:offset},(_,i)=><div key={`blank${i}`} className="bg-slate-950/80" />)}
      {Array.from({length:days},(_,i)=>{const d=i+1;const games=calendarGames.filter(g=>Number(g.month)===month&&Number(g.day)===i);return <div key={d} className="relative min-h-28 bg-slate-950 p-2 text-left align-top">
        <button type="button" aria-label={"Schedule for "+dateText(d)} disabled={busy} onClick={()=>openDay(d,games[0])} className="absolute inset-0 hover:bg-slate-900/40" />
        <span className="pointer-events-none relative text-xs text-slate-400">{d}</span>
        {games.map(g=>{const special=isAllStar(g);const atHome=Number(g.home_slot)===own;const other=view.slot_map.find(t=>Number(t.slot)===Number(atHome?g.away_slot:g.home_slot));return <button type="button" disabled={busy} onClick={()=>openDay(d,g)} aria-label={special?"All-Star Game "+g.home+" vs "+g.away:(atHome?"Home — vs ":"Away — @ ")+(other?.name??"Unresolved opponent")} key={g.record} className={"relative mt-1 flex w-full items-center gap-1 rounded border p-1 text-left "+(special?"border-yellow-500 bg-yellow-950/60 text-yellow-200":atHome?"border-sky-800 bg-sky-950/60":"border-rose-800 bg-rose-950/40")}>
          {!special&&other&&<OpponentLogo projectId={projectId} teamId={Number(other.team_id)} name={other.name}/>}<span className="min-w-0 text-[11px]"><span className="block font-semibold">{special?"All-Star Game":atHome?"Home — vs":"Away — @"}</span>{special?g.home+" vs "+g.away:other?.name??"Unresolved opponent"}</span></button>;})}
      </div>;})}
    </div></div>
    <dialog ref={dialog} onCancel={e=>{e.preventDefault();if(!busy)setDay(null);}} className="fixed inset-0 m-auto w-[min(36rem,calc(100vw-2rem))] rounded-xl border border-slate-700 bg-slate-900 p-5 text-slate-100 backdrop:bg-black/60" aria-label="Edit scheduled games">
      <h3 className="mb-3 text-lg font-semibold">Games on {day!=null?dateText(day):""}</h3>
      <div className="mb-4 space-y-2">{(snapshot?.games??[]).filter(g=>(Number(g.home_slot)===own||Number(g.away_slot)===own||isAllStar(g))&&Number(g.month)===month&&Number(g.day)===day!-1).map(g=><button type="button" disabled={busy} key={g.record} onClick={()=>choose(g)} className="block w-full rounded border border-slate-600 p-2 text-left text-sm hover:border-sky-400">{g.home} vs {g.away} · Edit / Remove</button>)}<Button variant="secondary" disabled={busy} onClick={()=>choose()}>Add game</Button></div>
      <fieldset disabled={busy} className="space-y-3"><label className="block text-sm">Date <input type="date" aria-label="Game date" value={date} onChange={e=>setDate(e.target.value)} className="ml-2 rounded border border-slate-600 bg-slate-950 p-2" /></label>
        <label className="block text-sm">Opponent <select aria-label="Opponent" value={opponent} onChange={e=>setOpponent(e.target.value)} className="ml-2 max-w-full rounded border border-slate-600 bg-slate-950 p-2"><option value="">Select team</option>{snapshot?.slot_map.filter(t=>Number(t.team_id)!==editTeam).map(t=><option key={String(t.team_id)} value={String(t.team_id)}>{t.name}</option>)}</select></label>
        <label className="block text-sm">Location <select aria-label="Home or away" value={home?"home":"away"} onChange={e=>setHome(e.target.value==="home")} className="ml-2 rounded border border-slate-600 bg-slate-950 p-2"><option value="home">Home</option><option value="away">Away</option></select></label>
      </fieldset>
      {error&&<p role="alert" className="my-3 text-sm text-rose-300">{error}</p>}
      <div className="mt-4 flex justify-end gap-2"><Button variant="secondary" disabled={busy} onClick={()=>setDay(null)}>Cancel</Button>{editing!=null&&<Button variant="secondary" disabled={busy} onClick={()=>void apply(true)}>Remove game</Button>}<Button disabled={busy||!opponent} onClick={()=>void apply(false)}>{busy?"Saving…":editing==null?"Add game":"Save game"}</Button></div>
    </dialog>
  </div>;
}

import { useEffect, useRef, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { queueFieldWrite } from "../lib/fieldWrites";
import { refreshFields } from "../lib/useFieldEdit";

export function TeamArtIdSelect({projectId,teamId}:{projectId:number;teamId:number}) {
  const qc=useQueryClient();const {update}=useProject();
  const mounted=useRef(true);
  useEffect(()=>{mounted.current=true;return ()=>{mounted.current=false;};},[]);
  const options=useQuery({queryKey:["team-artid-options",projectId],queryFn:()=>call("team_artid_options",{projectId})});
  const [pending,setPending]=useState<number|null>(null);
  const [error,setError]=useState<string|null>(null);
  const rows=options.data??[];
  const current=rows.find(([id])=>id===teamId);
  const unavailable=(art:number)=>[255,1000,1001,1002,1003,1004].includes(art)||rows.filter(([,id])=>id===art).length>1;
  async function select(other:number){
    if(other===teamId)return;
    setPending(other);setError(null);
    try {
      await queueFieldWrite(projectId,()=>call("team_artid_swap",{projectId,teamA:teamId,teamB:other}));
      // Show the new owners immediately; the shared refresh also updates logos and Undo.
      qc.setQueryData<[number,number,string][]>(["team-artid-options",projectId],old=>{
        const a=old?.find(([id])=>id===teamId),b=old?.find(([id])=>id===other);
        return old?.map(([id,art,name])=>[id,id===teamId&&b?b[1]:id===other&&a?a[1]:art,name]);
      });
      setPending(null);
      await refreshFields(qc,projectId,summary=>{if(mounted.current)update(summary);});
    } catch(e) {setError(String(e));setPending(null);}
  }
  return <div className="min-w-0">
    <label className="block text-xs text-slate-400">Art ID
      <select aria-label="Art ID" className="mt-1 w-full rounded border border-slate-700 bg-slate-950 px-2 py-1 text-sm text-slate-100 disabled:opacity-50"
        value={pending??teamId} disabled={!current||pending!==null||!!options.error||(current&&unavailable(current[1]))}
        onChange={e=>void select(Number(e.target.value))}>
        {!current&&<option value={teamId}>{options.isLoading?"Loading…":"Unavailable"}</option>}
        {rows.map(([id,art,name])=><option key={id} value={id} disabled={id!==teamId&&unavailable(art)}>{art} — {name}{unavailable(art)?" (unavailable)":""}</option>)}
      </select>
    </label>
    {(error||options.error)&&<p role="alert" className="mt-1 text-xs text-rose-300">{String(error??options.error)}</p>}
  </div>;
}

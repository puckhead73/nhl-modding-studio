import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { Button, Card } from "./primitives";

// Keys present in NHL07's own dictionary. Numeric bindings are inferred from stock
// membership / the executable's literal order; consuming game screens remain untraced.
export function Nhl07CompetitionNames({projectId,code,dictionarySha,onEdit}:{projectId:number;code:number;dictionarySha:string|null;onEdit:(key:string)=>void}) {
  const entries:[string,string][] = code===0?[
    ["League","NHL"],["Western Conference","WesternConference"],["Eastern Conference","EasternConference"],
    ["Northeast Division","NortheastDivision"],["Atlantic Division","AtlanticDivision"],["Southeast Division","SoutheastDivision"],
    ["Central Division","CentralDivision"],["Northwest Division","NorthwestDivision"],["Pacific Division","PacificDivision"],
  ]:code===1?[["League","NATIONAL"]]:[];
  const result=useQuery({queryKey:["competition-text",projectId,dictionarySha,code],queryFn:()=>call("dictionary_texts",{projectId,keys:entries.map(([,key])=>key)}),enabled:!!dictionarySha&&entries.length>0,retry:false});
  return <Card title="Competition names and presentation">
    {entries.length>0?<>
      <p className="mb-3 text-xs text-slate-400">Text from this project's dictionary. Which game screens use these entries still needs verification.</p>
      <table className="w-full text-left text-sm"><thead className="text-slate-400"><tr><th className="p-2">Name</th><th className="p-2">Current text</th><th className="p-2"></th></tr></thead>
        <tbody>{entries.map(([label,key],i)=><tr key={key} className="border-t border-slate-800"><td className="p-2">{label}</td><td className="p-2">{!dictionarySha?"Attach a dictionary to view":result.isLoading?"Loading…":result.error?<span role="alert" className="text-rose-300">{String(result.error)}</span>:result.data?.[i]??"No text"}</td><td className="p-2"><Button variant="secondary" disabled={result.data?.[i]==null} onClick={()=>onEdit(key)}>Edit text</Button></td></tr>)}</tbody>
      </table>
    </>:<p className="text-sm text-slate-400">This competition's name source has not been resolved in the NHL 07 dictionary.</p>}
    <p className="mt-3 text-xs text-slate-400">Available logo variants are below. Commentary assignments and changing which logo a competition uses still require mapping.</p>
  </Card>;
}

import { Link } from "react-router-dom";
import { EditableField } from "./EditableField";
import { ArenaAssetsCard } from "./ArenaAssetsCard";
import { ArenaVenueArtCard } from "./ArenaVenueArtCard";
import type { ArenaViewDto } from "../bindings/ArenaViewDto";

const groups: [string, [string, string][]][] = [
  ["Arena Details", [["arenaname","Name"],["cityname","City"],["maxattendance","Capacity"],["avgattendance","Average Attendance (%)"],["arenatype","Ice Surface"]]],
  ["Ice and Presentation", [["icelogo","Center Ice Logo"],["icelogoshown","Center Ice Logo Visibility"],["doorlogo","Door Logo"],["jumbotron","Jumbotron"],["puck","Puck"],["redline","Center Line"],["icead1","Ice Ad 1"],["icead2","Ice Ad 2"],["icead3","Ice Ad 3"],["boardad","Board Ads"],["boardadshown","Board Ad Visibility"]]],
  ["Audio Assignments", [["goalhorn","Goal Horn"],["periodhorn","Period Horn"],["chantaudioid","Crowd Chant"],["cityaudioid","City Commentary"],["arenaaudioid","Arena Commentary"]]],
  ["Venue Appearance", [["roof","Roof"],["walls","Walls"],["railings","Railings"],["step","Steps"],["boardtrim","Board Trim"],["seatlow","Lower Seats"],["seatmedium","Middle Seats"],["seathigh","Upper Seats"]]],
];

/** Stock venue assignments are shared by every team using the same source arena. */
export function Nhl07ArenaEditor({projectId,arena}:{projectId:number;arena:ArenaViewDto}) {
  const field=(name:string)=>arena.all_fields.find(f=>f.name===name);
  const used=new Set(groups.flatMap(([,fields])=>fields.map(([name])=>name)));
  const other=arena.all_fields.filter(f=>!used.has(f.name));
  return <div className="max-w-5xl space-y-4">
    <header className="rounded-lg border border-slate-700 bg-slate-900 p-4">
      <h1 className="text-2xl font-semibold">{field("arenaname")?.display||"Arena"}</h1>
      <p className="mt-1 text-sm text-slate-400">{field("cityname")?.display} · {Number(field("maxattendance")?.decoded??0).toLocaleString()} seats</p>
      <h2 className="mt-3 text-sm font-semibold">Teams playing here</h2>
      <div className="mt-1 flex flex-wrap gap-3">{arena.teams.map(team=><Link key={team.team_id} to={`/app/database/teams/${team.team_id}`} className="text-sm text-sky-300">{team.name}</Link>)}</div>
      {arena.teams.length>1&&<p className="mt-2 text-sm text-amber-300">Changes to this arena affect all teams listed here.</p>}
      <p className="mt-2 text-xs text-slate-400">Change a team's home arena from its Team Data tab.</p>
    </header>
    {groups.map(([title,fields])=><section key={title} className="rounded-lg border border-slate-800 bg-slate-900/40 p-4">
      <h2 className="mb-3 text-sm font-semibold">{title}</h2>
      {title!=="Arena Details"&&<p className="mb-3 text-xs text-slate-400">Choose an existing option by the venue that uses it. Check its appearance or sound in game.</p>}
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">{fields.map(([name,label])=>{const f=field(name);return f?<EditableField key={name} projectId={projectId} record={arena.record} field={f} label={label}/>:null;})}</div>
    </section>)}
    <details className="rounded-lg border border-slate-800 p-4"><summary className="cursor-pointer text-sm">Source assets</summary>
      <div className="mt-3 space-y-3"><ArenaAssetsCard projectId={projectId} arenaId={arena.arena_id}/><ArenaVenueArtCard projectId={projectId} arenaId={arena.arena_id}/></div>
    </details>
    <details className="rounded-lg border border-slate-800 p-4"><summary className="cursor-pointer text-sm">Additional information</summary>
      <p className="my-3 text-sm text-slate-400">Creating or duplicating arenas is not yet supported for NHL 07. The settings below are read-only.</p>
      <dl className="grid gap-2 text-xs sm:grid-cols-2">{other.map(f=><div key={f.studio_field}><dt className="text-slate-400">{f.name}</dt><dd>{f.display}</dd></div>)}</dl>
    </details>
  </div>;
}

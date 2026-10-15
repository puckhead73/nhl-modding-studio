// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
const calls: {cmd:string;args:any}[]=[];
let fail=false;
const choice=(value:number,label:string)=>({value,label,evidence:"inferred"});
const group={league_code:0,conferences:[choice(0,"Western Conference"),choice(1,"Eastern Conference")],divisions:[choice(0,"Northeast Division"),choice(3,"Central Division")],pairs:[[1,0],[0,3]]};
const teams=[{team_id:1,name:"Boston",league:0},{team_id:2,name:"Sweden",league:1},{team_id:30,name:"East Stars",league:0}];
vi.mock("../lib/project",()=>({useProject:()=>({update:vi.fn(),install:null,iso:null})}));
vi.mock("../lib/useFieldEdit",()=>({refreshFields:async()=>{}}));
vi.mock("./TextureReplace",()=>({useTextureReplace:()=>({}),TextureThumbnail:()=>null}));
vi.mock("./ArchiveMemberReplace",()=>({useArchiveMemberReplace:()=>({})}));
vi.mock("../lib/client",()=>({call:async(cmd:string,args:any)=>{
 calls.push({cmd,args});
 if(cmd==="list_teams")return teams;
 if(cmd==="team_membership_options")return {current:{league_code:0,conference:choice(1,"Eastern Conference"),division:choice(0,"Northeast Division")},leagues:[choice(0,"NHL"),choice(1,"National")],groups_by_league:[group,{...group,league_code:1}],blocker:null};
 if(cmd==="dictionary_texts")return args.keys.map((key:string)=>key==="NHL"?"Custom League":key);
 if(cmd==="league_logo"){if(fail)throw Error("Logo read failed");return {art_index:0,evidence:"inferred",witness:"Stock mapping",shared_with:[],variants:[],notes:["Technical note one","Technical note two","Technical note three"]};}
 if(fail)throw Error("Move refused");
 return {};
}}));
import { Nhl07LeagueStructure } from "./Nhl07LeagueStructure";
import { Nhl07CompetitionNames } from "./Nhl07CompetitionNames";
import { LeagueLogoCard } from "./LeagueLogoCard";
function mount(ui:React.ReactNode){return render(<QueryClientProvider client={new QueryClient({defaultOptions:{queries:{retry:false}}})}><MemoryRouter>{ui}</MemoryRouter></QueryClientProvider>);}
beforeEach(()=>{calls.length=0;fail=false;});afterEach(cleanup);
it("keeps changes local until Apply, restricts division choices and supports Cancel",async()=>{
 mount(<Nhl07LeagueStructure projectId={7} members={[teams[0]] as any}/>);
 const conf=await screen.findByLabelText("Boston conference");
 fireEvent.change(conf,{target:{value:"0"}});
 expect((screen.getByLabelText("Boston division") as HTMLSelectElement).value).toBe("3");
 expect(calls.some(c=>c.cmd==="set_team_membership")).toBe(false);
 fireEvent.click(screen.getByText("Cancel"));expect((conf as HTMLSelectElement).value).toBe("1");
 fireEvent.change(conf,{target:{value:"0"}});fireEvent.click(screen.getByText("Apply"));
 await waitFor(()=>expect(calls.find(c=>c.cmd==="set_team_membership")?.args).toEqual({projectId:7,teamId:1,leagueCode:0,conference:0,division:3}));
});
it("requires a destination team for a league swap and preserves a refused draft",async()=>{
 mount(<Nhl07LeagueStructure projectId={7} members={[teams[0]] as any}/>);
 fireEvent.change(await screen.findByLabelText("Boston league"),{target:{value:"1"}});
 expect((screen.getByText("Swap") as HTMLButtonElement).disabled).toBe(true);
 fireEvent.change(screen.getByLabelText("Boston swap with"),{target:{value:"2"}});fail=true;
 fireEvent.click(screen.getByText("Swap"));expect((await screen.findByRole("alert")).textContent).toContain("Move refused");
 expect(calls.find(c=>c.cmd==="swap_team_memberships")?.args).toEqual({projectId:7,first:1,second:2});
 expect((screen.getByLabelText("Boston swap with") as HTMLSelectElement).value).toBe("2");
});
it("protects All-Star teams",async()=>{
 mount(<Nhl07LeagueStructure projectId={7} members={[teams[2]] as any}/>);
 expect((await screen.findByLabelText("East Stars league") as HTMLSelectElement).disabled).toBe(true);
 expect(screen.queryByText("Apply")).toBeNull();
});
it("reads current dictionary text without editing and opens the chosen key",async()=>{
 const edit=vi.fn();mount(<Nhl07CompetitionNames projectId={7} code={0} dictionarySha="abc" onEdit={edit}/>);
 await screen.findByText("Custom League");fireEvent.click(screen.getAllByText("Edit text")[0]);expect(edit).toHaveBeenCalledWith("NHL");
 expect(calls).toHaveLength(1);expect(calls[0].cmd).toBe("dictionary_texts");expect(calls[0].args.keys).toHaveLength(9);
});
it("does not invent a European name binding",()=>{
 mount(<Nhl07CompetitionNames projectId={7} code={2} dictionarySha="abc" onEdit={vi.fn()}/>);
 expect(screen.getByText(/name source has not been resolved/)).toBeTruthy();expect(calls).toHaveLength(0);
});
it("removes successful logo notes but retains read errors",async()=>{
 const view=mount(<LeagueLogoCard projectId={7} leagueCode={0}/>);await screen.findByText("Stock mapping");
 expect(screen.queryByText("Technical note one")).toBeNull();view.unmount();fail=true;
 mount(<LeagueLogoCard projectId={7} leagueCode={0}/>);expect((await screen.findByRole("alert")).textContent).toContain("Logo read failed");
});

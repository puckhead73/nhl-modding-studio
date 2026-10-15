// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { MemoryRouter } from "react-router-dom";
import { Nhl07ArenaEditor } from "./Nhl07ArenaEditor";
import type { ArenaViewDto } from "../bindings/ArenaViewDto";
import type { FieldViewDto } from "../bindings/FieldViewDto";

const mutate=vi.hoisted(()=>vi.fn());
vi.mock("../lib/useFieldEdit",()=>({useFieldEdit:(_id:number,_row:number,field:FieldViewDto)=>({field,mutate,error:null})}));
vi.mock("../lib/project",()=>({useProject:()=>({project:{project_id:1},update:vi.fn()})}));
vi.mock("./ArenaAssetsCard",()=>({ArenaAssetsCard:()=>null}));
vi.mock("./ArenaVenueArtCard",()=>({ArenaVenueArtCard:()=>null}));
afterEach(()=>{cleanup();vi.clearAllMocks();});
function field(name:string,display:string,extra:Partial<FieldViewDto>={}):FieldViewDto {
 return {studio_table:"cubz",studio_field:name,name,display,editable:true,is_string:false,decoded:0,enum_options:[],range_low:0,range_high:32767,...extra} as FieldViewDto;
}
function mount(){const arena={arena_id:14,record:14,identity:[],teams:[{team_id:14,record:14,name:"Minnesota",abbrev:"MIN",league:0,league_label:"NHL"},{team_id:24,record:24,name:"St. Louis",abbrev:"STL",league:0,league_label:"NHL"}],all_fields:[
 field("arenaname","Review Arena",{is_string:true}),field("cityname","St. Paul",{is_string:true}),field("maxattendance","19000",{decoded:19000}),
 field("arenatype","North American ice",{enum_options:[{raw:0,label:"North American ice"},{raw:1,label:"International ice"}]}),
 field("goalhorn","Vancouver venue — option 28",{decoded:28,enum_options:[{raw:14,label:"Minnesota venue — option 14"},{raw:28,label:"Vancouver venue — option 28"}]}),field("index","14",{editable:false}),
 ]} as ArenaViewDto;render(<MemoryRouter><Nhl07ArenaEditor projectId={1} arena={arena}/></MemoryRouter>);}
it("names every shared tenant and links to the team assignment page",()=>{
 mount();expect(screen.getByText("Changes to this arena affect all teams listed here.")).toBeTruthy();
 expect(screen.getByRole("link",{name:"St. Louis"}).getAttribute("href")).toBe("/app/database/teams/24");
 expect(screen.getByText("Creating or duplicating arenas is not yet supported for NHL 07. The settings below are read-only.")).toBeTruthy();
});
it("keeps readable ice and audio choices while editing and submits the selected IDs",()=>{
 mount();fireEvent.click(screen.getByRole("button",{name:"North American ice"}));
 fireEvent.change(screen.getByRole("combobox",{name:"Ice Surface"}),{target:{value:"1"}});expect(mutate).toHaveBeenLastCalledWith({intValue:1});
 fireEvent.click(screen.getByRole("button",{name:"Vancouver venue — option 28"}));
 expect(screen.getByRole("option",{name:"Minnesota venue — option 14"})).toBeTruthy();
 fireEvent.change(screen.getByRole("combobox",{name:"Goal Horn"}),{target:{value:"14"}});expect(mutate).toHaveBeenLastCalledWith({intValue:14});
});

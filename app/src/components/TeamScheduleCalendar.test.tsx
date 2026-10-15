// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter } from "react-router-dom";
import { TeamScheduleCalendar } from "./TeamScheduleCalendar";

const mocks=vi.hoisted(()=>({call:vi.fn(),refresh:vi.fn()}));
vi.mock("../lib/client",()=>({call:mocks.call}));
vi.mock("../lib/project",()=>({useProject:()=>({update:vi.fn()}),useAssetRootArgs:()=>({installDir:null,rootArgs:{}})}));
vi.mock("../lib/useFieldEdit",()=>({refreshFields:mocks.refresh}));
vi.mock("../lib/fieldWrites",()=>({queueFieldWrite:(_id:number,work:()=>unknown)=>work()}));
const fixture={table:{studio_table:"ksGb",name:"nhlschedulelongnoleap",league_code:0},fingerprint:"current",
  slot_map:[{slot:0,team_id:14,name:"Minnesota"},{slot:1,team_id:28,name:"Vancouver"},{slot:2,team_id:24,name:"St. Louis"}],
  games:[{record:0,index:0,month:9,day:3,home_slot:0,away_slot:1,home:"Minnesota",away:"Vancouver",round:0,status:0}],fields:[],field_widths:[],findings:[],slots:[],unresolved_slots:[]};
beforeEach(()=>{
  vi.clearAllMocks();mocks.refresh.mockResolvedValue(undefined);
  HTMLDialogElement.prototype.showModal=function(){this.setAttribute("open","");};
  HTMLDialogElement.prototype.close=function(){this.removeAttribute("open");};
  mocks.call.mockImplementation(async(command:string)=>command==="schedule_tables"?[fixture.table]:command==="schedule_view"?structuredClone(fixture):null);
});
afterEach(cleanup);
it("opens the clicked game already selected, including a second fixture on the day",async()=>{
  const data=structuredClone(fixture);
  data.games.push({...data.games[0],record:1,index:1,away_slot:2,away:"St. Louis"});
  mocks.call.mockImplementation(async(command:string)=>command==="schedule_tables"?[data.table]:command==="schedule_view"?data:null);
  mount();fireEvent.click(await screen.findByRole("button",{name:/Home — vs St. Louis/}));
  const dialog=screen.getByRole("dialog");
  expect((within(dialog).getByLabelText("Opponent") as HTMLSelectElement).value).toBe("24");
  expect(within(dialog).getByRole("button",{name:"Save game"})).toBeTruthy();
  expect(within(dialog).getByRole("button",{name:"Remove game"})).toBeTruthy();
});
it("shows the shared All-Star fixture in gold without adding it to the club's game count",async()=>{
  const data=structuredClone(fixture);
  data.slot_map.push({slot:30,team_id:30,name:"East"},{slot:31,team_id:31,name:"West"});
  data.games.push({...data.games[0],record:1,index:1,home_slot:30,away_slot:31,home:"East",away:"West"});
  mocks.call.mockImplementation(async(command:string)=>command==="schedule_tables"?[data.table]:command==="schedule_view"?data:command==="team_schedule_edit"?data:null);
  mount();const card=await screen.findByRole("button",{name:/All-Star Game East vs West/});
  expect(card.className).toContain("border-yellow-500");
  expect(screen.getByText(/^1 games/)).toBeTruthy();fireEvent.click(card);
  const dialog=screen.getByRole("dialog");
  expect((within(dialog).getByLabelText("Opponent") as HTMLSelectElement).value).toBe("31");
  fireEvent.click(within(dialog).getByRole("button",{name:"Save game"}));
  await waitFor(()=>expect(mocks.call).toHaveBeenCalledWith("team_schedule_edit",expect.objectContaining({team:30,opponent:31,record:1})));
});
function mount(teamId=14){const qc=new QueryClient({defaultOptions:{queries:{retry:false,staleTime:Infinity}}});render(<QueryClientProvider client={qc}><MemoryRouter><TeamScheduleCalendar projectId={1} teamId={teamId} league={0}/></MemoryRouter></QueryClientProvider>);return qc;}
it("shows the same fixture as home or away with readable opponent fallback",async()=>{
  mount();expect(await screen.findByText("Home — vs")).toBeTruthy();expect(screen.getByText("Vancouver")).toBeTruthy();
  cleanup();mount(28);expect(await screen.findByText("Away — @")).toBeTruthy();expect(screen.getByText("Minnesota")).toBeTruthy();
});
it("opens a day centrally and cancelling makes no write",async()=>{
  mount();fireEvent.click(await screen.findByRole("button",{name:"Schedule for 2006-10-04"}));
  const dialog=screen.getByRole("dialog");expect(dialog.className).toContain("inset-0");
  fireEvent.click(within(dialog).getByRole("button",{name:"Cancel"}));
  expect(mocks.call.mock.calls.some(c=>c[0]==="team_schedule_edit")).toBe(false);
  expect(screen.queryByRole("dialog")).toBeNull();
});
it("echoes an added fixture immediately and restores it on refusal",async()=>{
  const qc=mount();fireEvent.click(await screen.findByRole("button",{name:"Schedule for 2006-10-05"}));
  let reject!:(reason:string)=>void;
  mocks.call.mockImplementation((command:string)=>command==="team_schedule_edit"?new Promise((_yes,no)=>{reject=no;}):Promise.resolve(null));
  const dialog=screen.getByRole("dialog");fireEvent.change(within(dialog).getByLabelText("Opponent"),{target:{value:"24"}});
  fireEvent.click(within(dialog).getAllByRole("button",{name:"Add game"}).slice(-1)[0]);
  await waitFor(()=>expect(mocks.call).toHaveBeenCalledWith("team_schedule_edit",expect.objectContaining({team:14,opponent:24,month:10,day:5,record:null,expectedGames:expect.any(Array)})));
  expect(qc.getQueryData<typeof fixture>(["schedule-view",1,"ksGb"])?.games.length).toBe(2);
  await act(async()=>reject("Opponent already plays that day"));
  expect(qc.getQueryData<typeof fixture>(["schedule-view",1,"ksGb"])?.games.length).toBe(1);
  expect(screen.getByRole("alert").textContent).toContain("Opponent already plays");
});
it("moves through months and never submits dates outside the displayed season",async()=>{
  mount();await screen.findByRole("button",{name:"Schedule for 2006-10-04"});fireEvent.click(screen.getByRole("button",{name:"Previous month"}));
  fireEvent.click(screen.getByRole("button",{name:"Schedule for 2006-09-01"}));const dialog=screen.getByRole("dialog");
  fireEvent.change(within(dialog).getByLabelText("Opponent"),{target:{value:"24"}});fireEvent.change(within(dialog).getByLabelText("Game date"),{target:{value:"2026-09-01"}});
  fireEvent.click(within(dialog).getAllByRole("button",{name:"Add game"}).slice(-1)[0]);
  expect((await screen.findByRole("alert")).textContent).toContain("displayed season");expect(mocks.call.mock.calls.some(c=>c[0]==="team_schedule_edit")).toBe(false);
});

// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
const call=vi.hoisted(()=>vi.fn());
vi.mock("../lib/client",()=>({call}));
vi.mock("../lib/project",()=>({useProject:()=>({update:vi.fn()})}));
vi.mock("../lib/useFieldEdit",()=>({refreshFields:async()=>{}}));
import { TeamArtIdSelect } from "./TeamArtIdSelect";
const rows:[number,number,string][]=[[1,10,"Boston"],[2,20,"Toronto"],[3,30,"Shared A"],[4,30,"Shared B"]];
function mount(){const qc=new QueryClient({defaultOptions:{queries:{retry:false}}});render(<QueryClientProvider client={qc}><TeamArtIdSelect projectId={7} teamId={1}/></QueryClientProvider>);return qc;}
beforeEach(()=>{call.mockReset();call.mockImplementation(async(cmd:string)=>cmd==="team_artid_options"?rows:{});});afterEach(cleanup);
it("labels IDs with their owners and automatically swaps on selection",async()=>{
 const qc=mount();await screen.findByText("20 — Toronto");
 fireEvent.change(screen.getByLabelText("Art ID"),{target:{value:"2"}});
 await waitFor(()=>expect(call).toHaveBeenCalledWith("team_artid_swap",{projectId:7,teamA:1,teamB:2}));
 await screen.findByText("20 — Boston");expect(screen.getByText("10 — Toronto")).toBeTruthy();
 // An Undo refresh replaces both labels from the server state.
 qc.setQueryData(["team-artid-options",7],rows);await screen.findByText("10 — Boston");
 expect((screen.getByLabelText("Art ID") as HTMLSelectElement).value).toBe("1");
});
it("leaves the original ID selected when a swap is refused",async()=>{
 call.mockImplementation(async(cmd:string)=>{if(cmd==="team_artid_options")return rows;throw Error("Swap refused");});
 mount();await screen.findByText("20 — Toronto");fireEvent.change(screen.getByLabelText("Art ID"),{target:{value:"2"}});
 expect((await screen.findByRole("alert")).textContent).toContain("Swap refused");
 expect((screen.getByLabelText("Art ID") as HTMLSelectElement).value).toBe("1");
 expect(screen.getByText("10 — Boston")).toBeTruthy();
});
it("shows shared IDs but disables ambiguous swaps",async()=>{
 mount();expect((await screen.findByText("30 — Shared A (unavailable)") as HTMLOptionElement).disabled).toBe(true);
 expect(call.mock.calls.every(([cmd])=>cmd==="team_artid_options")).toBe(true);
});

// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, expect, it, vi } from "vitest";
import { ExtractionPanel } from "./ExtractionPanel";
const mocks=vi.hoisted(()=>({call:vi.fn()}));
vi.mock("../lib/client",()=>({call:mocks.call}));
afterEach(()=>{cleanup();mocks.call.mockReset();});
const pre={root:"S:/copy",state:"needed",reason:"Files need extraction",archives:[],collisions:[],members_present:0,members_total:2,members_to_write:2,bytes_to_write:10,free_bytes:100};
const done={job:1,root:pre.root,state:"done",current:"",archives_done:2,archives_total:2,files_written:1,files_retained:0,files_conflicting:0,files_blocked:1,files_unsupported:0,files_unsafe:0,archives_failed:0,bytes_written:5,problems:[]};
function mount(onDone=vi.fn()) {
  const client=new QueryClient({defaultOptions:{queries:{retry:false},mutations:{retry:false}}});
  return {...render(<QueryClientProvider client={client}><ExtractionPanel root={pre.root} platform="ps4" mode="onboarding" onDone={onDone}/></QueryClientProvider>),onDone};
}
it("waits for the post-extraction scan and reports partial completion truthfully",async()=>{
  let finishScan!: (value:unknown)=>void;
  let scans=0;
  mocks.call.mockImplementation((method:string)=>{
    if(method==="extract_running_for")return Promise.resolve(null);
    if(method==="extract_start")return Promise.resolve(done);
    if(method==="extract_preflight")return ++scans===1?Promise.resolve(pre):new Promise(r=>{finishScan=r;});
    throw Error(method);
  });
  const {onDone}=mount();
  const button=await screen.findByRole("button",{name:"Extract now"});
  await waitFor(()=>expect((button as HTMLButtonElement).disabled).toBe(false));
  fireEvent.click(button);
  await screen.findByText("Extraction finished. Some files were skipped.");
  await waitFor(()=>expect(finishScan).toBeDefined());
  expect(onDone).not.toHaveBeenCalled();
  const after={...pre,state:"blocked",members_present:1,members_to_write:0,reason:"Remaining files have conflicting owners"};
  finishScan(after);
  await waitFor(()=>expect(onDone).toHaveBeenCalledExactlyOnceWith(done,after));
  fireEvent.click(await screen.findByRole("button", { name: "Review skipped files" }));
  expect((screen.getByRole("button",{name:"Extract now"}) as HTMLButtonElement).disabled).toBe(true);
});
it("does not hand an old scan back to a wizard that has unmounted",async()=>{
  let finishScan!: (value:unknown)=>void;
  mocks.call.mockImplementation((method:string)=>method==="extract_running_for"?Promise.resolve(done):new Promise(r=>{finishScan=r;}));
  const {onDone,unmount}=mount();
  await screen.findByText("Extraction finished. Some files were skipped.");
  await waitFor(()=>expect(finishScan).toBeDefined());
  unmount(); finishScan({...pre,state:"blocked"});
  await new Promise(r=>setTimeout(r,0));
  expect(onDone).not.toHaveBeenCalled();
});
it("counts containers needing extraction separately from completed and unreadable containers", async () => {
  mocks.call.mockImplementation((method: string) => Promise.resolve(method === "extract_running_for" ? null : {
    ...pre, state: "partial", archives: [{state:"pending"}, {state:"partial"}, {state:"complete"}, {state:"unsupported"}],
  }));
  mount();
  await screen.findByText("2 containers need extracting.");
  expect(screen.getByText("1 container could not be read and will be skipped.")).toBeTruthy();
  expect(screen.queryByText("existing files detected")).toBeNull();
  expect(screen.queryByText(/Destination:/)).toBeNull();
  expect(screen.queryByText(/member paths/)).toBeNull();
});

it("shows measured file progress and replaces the extraction prompt after success", async () => {
  mocks.call.mockImplementation((method: string) => Promise.resolve(method === "extract_running_for" ? null : method === "extract_start" ? {
    ...done, files_blocked: 0, state: "running", current_files_done: 1, current_files_total: 2, files_done: 1, files_total: 4,
  } : method === "extract_status" ? { ...done, files_blocked: 0 } : pre));
  const original = mocks.call.getMockImplementation()!;
  let scans = 0;
  mocks.call.mockImplementation((method: string, ...args: unknown[]) => method === "extract_preflight" && ++scans > 1 ? Promise.resolve({ ...pre, state: "complete", members_to_write: 0 }) : original(method, ...args));
  mount();
  const button = await screen.findByRole("button", { name: "Extract now" });
  await waitFor(() => expect((button as HTMLButtonElement).disabled).toBe(false));
  fireEvent.click(button);
  const progress = await screen.findByRole("progressbar", { name: "Current container" });
  expect(progress.getAttribute("value")).toBe("1"); expect(progress.getAttribute("max")).toBe("2");
  expect(screen.getByRole("progressbar", { name: "Overall extraction" }).getAttribute("max")).toBe("4");
  await screen.findByText("Extraction completed");
  expect(screen.queryByRole("button", { name: "Extract now" })).toBeNull();
});

it("passes an edited extraction destination to the native job", async () => {
  mocks.call.mockImplementation((method: string) => Promise.resolve(method === "extract_running_for" ? null : method === "extract_start" ? done : pre));
  mount();
  fireEvent.change(await screen.findByLabelText("Extraction destination"), { target: { value: "D:/Assets/NHL07" } });
  const button = screen.getByRole("button", { name: "Extract now" });
  await waitFor(() => expect((button as HTMLButtonElement).disabled).toBe(false));
  fireEvent.click(button);
  await waitFor(() => expect(mocks.call).toHaveBeenCalledWith("extract_start", { root: pre.root, platform: "ps4", destination: "D:/Assets/NHL07" }));
});

it("keeps extraction available when a finished job still has missing outputs", async () => {
  mocks.call.mockImplementation((method: string) => Promise.resolve(method === "extract_running_for" ? null : method === "extract_start" ? { ...done, files_blocked: 0 } : pre));
  mount();
  fireEvent.click(await screen.findByRole("button", { name: "Extract now" }));
  await waitFor(() => expect(screen.getByRole("button", { name: "Extract now" })).toBeTruthy());
  expect(screen.queryByText("Your game assets are ready for editing.")).toBeNull();
});

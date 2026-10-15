// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, expect, it, vi } from "vitest";
import { VanillaCheck } from "./VanillaCheck";
const mocks = vi.hoisted(() => ({ call: vi.fn() }));
vi.mock("../lib/client", () => ({ call: mocks.call }));
afterEach(() => { cleanup(); mocks.call.mockReset(); });
const reference = { status: "vanilla", reference: "NHL 07 USA", region: "USA", matched: 12, loose: 10, containers: 2, findings: [] };
function mount() {
  const onReady = vi.fn();
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const tree = (root: string) => <QueryClientProvider client={client}><VanillaCheck root={root} gameId="nhl07" platform="xbox" onReady={onReady} /></QueryClientProvider>;
  return { ...render(tree("E:/Game")), onReady, tree };
}
it("allows matching original loose files without a warning", async () => {
  mocks.call.mockResolvedValue(reference);
  const { onReady } = mount();
  await waitFor(() => expect(onReady).toHaveBeenLastCalledWith(true));
  expect(screen.queryByRole("checkbox")).toBeNull();
  expect(screen.getByText("Vanilla")).toBeTruthy();
  expect(screen.getByText("Region: USA")).toBeTruthy();
  expect(screen.queryByText(/files shipped loose/)).toBeNull();
});
it.each(["different", "additional", "unverified"])("allows %s after acknowledgement and resets it for another folder", async status => {
  mocks.call.mockResolvedValue({ ...reference, status });
  const { onReady, rerender, tree } = mount();
  const checkbox = await screen.findByRole("checkbox");
  expect(screen.getByText(status === "unverified" ? "Unknown" : "Modified")).toBeTruthy();
  expect(screen.getByText(status === "additional" ? "Region: USA" : "Region: Unknown")).toBeTruthy();
  expect(onReady).toHaveBeenLastCalledWith(false);
  fireEvent.click(checkbox);
  await waitFor(() => expect(onReady).toHaveBeenLastCalledWith(true));
  rerender(tree("E:/Other game"));
  await waitFor(() => expect(onReady).toHaveBeenLastCalledWith(false));
  expect((await screen.findByRole("checkbox") as HTMLInputElement).checked).toBe(false);
});
it("lets the user acknowledge a failed check without showing technical errors", async () => {
  mocks.call.mockRejectedValue(new Error("Cannot read files"));
  const { onReady } = mount();
  fireEvent.click(await screen.findByRole("checkbox"));
  await waitFor(() => expect(onReady).toHaveBeenLastCalledWith(true));
  expect(screen.getByText("Unknown")).toBeTruthy();
  expect(screen.getByText("Region: Unknown")).toBeTruthy();
  expect(screen.queryByText(/Cannot read files/)).toBeNull();
});

it("rescans the same source and requires fresh acknowledgement before continuing", async () => {
  mocks.call.mockResolvedValueOnce({ ...reference, status: "different" });
  let resolve!: (value: typeof reference) => void;
  mocks.call.mockImplementationOnce(() => new Promise(r => { resolve = r; }));
  const { onReady } = mount();
  fireEvent.click(await screen.findByRole("checkbox"));
  await waitFor(() => expect(onReady).toHaveBeenLastCalledWith(true));
  fireEvent.click(screen.getByRole("button", { name: "Rescan" }));
  await waitFor(() => expect(mocks.call).toHaveBeenCalledTimes(2));
  expect(mocks.call).toHaveBeenLastCalledWith("game_files_check", { root: "E:/Game", gameId: "nhl07", platform: "xbox" });
  expect(onReady).toHaveBeenLastCalledWith(false);
  expect((screen.getByRole("button", { name: "Rescan" }) as HTMLButtonElement).disabled).toBe(true);
  expect(screen.queryByRole("checkbox")).toBeNull();
  resolve({ ...reference, status: "different" });
  expect((await screen.findByRole("checkbox") as HTMLInputElement).checked).toBe(false);
  expect(onReady).toHaveBeenLastCalledWith(false);
  mocks.call.mockResolvedValueOnce(reference);
  fireEvent.click(screen.getByRole("button", { name: "Rescan" }));
  await waitFor(() => expect(onReady).toHaveBeenLastCalledWith(true));
  expect(screen.getByText("Vanilla")).toBeTruthy();
});

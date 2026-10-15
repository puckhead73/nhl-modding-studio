// @vitest-environment jsdom
import { cleanup, render, screen, waitFor, fireEvent } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter } from "react-router-dom";
import { afterEach, expect, it, vi } from "vitest";
const call = vi.hoisted(() => vi.fn());
vi.mock("../lib/client", () => ({ call }));
import { CorpusGate } from "./CorpusGate";
afterEach(() => { cleanup(); localStorage.clear(); call.mockReset(); });
function mount() {
  render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } })}><MemoryRouter><CorpusGate /></MemoryRouter></QueryClientProvider>);
}
it("automatically selects the bundled corpus without remembering a machine path", async () => {
  let opened = false;
  call.mockImplementation(async (cmd) => cmd === "corpus_open" ? opened : (opened = true, 190303));
  mount();
  await waitFor(() => expect(call).toHaveBeenCalledWith("open_corpus", { path: "" }));
  expect(localStorage.getItem("fhm_corpus_path")).toBeNull();
});
it("preserves an explicit path and lets a stale selection recover to the bundled corpus", async () => {
  localStorage.setItem("fhm_corpus_path", "D:/missing.db");
  call.mockImplementation(async (cmd, args) => {
    if (cmd === "corpus_open") return false;
    if (args.path) throw Error("Selected database is missing");
    return 190303;
  });
  mount();
  await screen.findByText(/Selected database is missing/);
  fireEvent.change(screen.getByLabelText("Historical database path"), { target: { value: "" } });
  fireEvent.click(screen.getByRole("button", { name: "Use bundled data" }));
  await waitFor(() => expect(localStorage.getItem("fhm_corpus_path")).toBeNull());
  expect(call).toHaveBeenCalledWith("open_corpus", { path: "" });
});

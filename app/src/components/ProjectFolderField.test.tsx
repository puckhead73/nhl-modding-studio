// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, expect, it, vi } from "vitest";
const mocks = vi.hoisted(() => ({ call: vi.fn(), open: vi.fn(), remember: vi.fn(), picker: vi.fn() }));
vi.mock("../lib/client", () => ({ call: mocks.call }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: mocks.picker }));
vi.mock("../lib/project", () => ({ useProject: () => ({ project: { project_id: 1 }, info: { dir: "D:/Projects/QA" }, open: mocks.open, update: vi.fn(), updateInfo: vi.fn() }) }));
vi.mock("../lib/settings", () => ({ useSettings: () => ({ rememberProject: mocks.remember }) }));
import { ProjectFolderField } from "./ProjectFolderField";
afterEach(() => { cleanup(); vi.resetAllMocks(); });
function mount() {
  render(<QueryClientProvider client={new QueryClient({ defaultOptions: { mutations: { retry: false } } })}><ProjectFolderField /></QueryClientProvider>);
}
it("saves, copies, acknowledges close and opens the chosen folder", async () => {
  mocks.picker.mockResolvedValue("E:/Storage");
  mocks.call.mockImplementation(async (cmd, args) => {
    if (cmd === "project_save") return { saved: true, summary: { project_id: 1 } };
    if (cmd === "project_info") return { dir: "D:/Projects/QA" };
    if (cmd === "project_copy_to") return args.destination;
    if (cmd === "close_project") return { outcome: "closed" };
    if (cmd === "project_open") return { summary: { project_id: 2 }, project: { dir: args.projectDir, working_main_db: "E:/Storage/QA/working/db/game.db" }, discovery: {} };
    throw new Error(cmd);
  });
  mount();
  fireEvent.click(screen.getByRole("button", { name: "Browse…" }));
  await waitFor(() => expect((screen.getByLabelText("Project folder") as HTMLInputElement).value).toBe("E:/Storage/QA"));
  fireEvent.click(screen.getByRole("button", { name: "Change folder" }));
  await waitFor(() => expect(mocks.open).toHaveBeenCalledOnce());
  expect(mocks.call.mock.calls.map(([cmd]) => cmd)).toEqual(["project_save", "project_info", "project_copy_to", "close_project", "project_open"]);
  expect(mocks.remember).toHaveBeenCalledWith("E:/Storage/QA");
});
it("keeps the original session when copying fails", async () => {
  mocks.call.mockImplementation(async cmd => {
    if (cmd === "project_save") return { saved: true, summary: {} };
    if (cmd === "project_info") return {};
    throw new Error("Destination exists");
  });
  mount();
  fireEvent.change(screen.getByLabelText("Project folder"), { target: { value: "E:/Existing" } });
  fireEvent.click(screen.getByRole("button", { name: "Change folder" }));
  expect((await screen.findByRole("alert")).textContent).toContain("Destination exists");
  expect(mocks.call.mock.calls.some(([cmd]) => cmd === "close_project")).toBe(false);
  expect(mocks.open).not.toHaveBeenCalled();
});

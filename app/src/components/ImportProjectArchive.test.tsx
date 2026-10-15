// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, expect, it, vi } from "vitest";
const mocks = vi.hoisted(() => ({ call: vi.fn(), open: vi.fn() }));
vi.mock("../lib/client", () => ({ call: mocks.call }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: mocks.open }));
import { ImportProjectArchive } from "./ImportProjectArchive";
afterEach(() => { cleanup(); vi.resetAllMocks(); });
function mount(done = vi.fn()) {
  render(<QueryClientProvider client={new QueryClient()}><ImportProjectArchive projectsRoot="D:/Projects" disabled={false} onImported={done} /></QueryClientProvider>);
  return done;
}
it("previews an archive, suggests the organized folder, and imports only after confirmation", async () => {
  mocks.open.mockResolvedValue("D:/Mods/Roster.nhlproject");
  mocks.call.mockImplementation(async cmd => cmd === "project_archive_preview" ? { name: "Roster", game_id: "nhl07", platform: "xbox" } : "D:/Projects/xbox360/nhl07/Roster");
  const done = mount();
  fireEvent.click(screen.getByText("Import developer project…"));
  const folder = await screen.findByLabelText("Imported project folder");
  expect((folder as HTMLInputElement).value).toBe("D:/Projects/xbox360/nhl07/Roster");
  expect(mocks.call).toHaveBeenCalledTimes(1);
  fireEvent.change(screen.getByLabelText("Imported game source folder"), { target: { value: "F:/NHL07" } });
  fireEvent.click(screen.getByText("Import and open"));
  await waitFor(() => expect(done).toHaveBeenCalledWith("D:/Projects/xbox360/nhl07/Roster"));
  expect(mocks.call).toHaveBeenLastCalledWith("project_import_archive", { archive: "D:/Mods/Roster.nhlproject", destination: "D:/Projects/xbox360/nhl07/Roster", sourceRoot: "F:/NHL07" });
});
it("does not import a cancelled picker or open a failed import", async () => {
  mocks.open.mockResolvedValueOnce(null).mockResolvedValueOnce("D:/Roster.nhlproject");
  const done = mount();
  fireEvent.click(screen.getByText("Import developer project…"));
  await waitFor(() => expect((screen.getByText("Import developer project…") as HTMLButtonElement).disabled).toBe(false));
  expect(mocks.call).not.toHaveBeenCalled();
  mocks.call.mockResolvedValueOnce({ name: "Roster", game_id: "nhl07", platform: "xbox" }).mockRejectedValueOnce(new Error("Existing folders are never overwritten"));
  fireEvent.click(screen.getByText("Import developer project…"));
  fireEvent.click(await screen.findByText("Import and open"));
  expect((await screen.findByRole("alert")).textContent).toContain("never overwritten");
  expect(done).not.toHaveBeenCalled();
});

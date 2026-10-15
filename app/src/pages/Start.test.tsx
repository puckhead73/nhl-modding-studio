// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { StartPage } from "./Start";
import { SaveEditorPage } from "./SaveEditor";

vi.mock("@tauri-apps/plugin-dialog", () => ({ open: () => Promise.resolve(null) }));
import { ModManagerPage } from "./ModManager";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ProjectProvider } from "../lib/project";

afterEach(cleanup);
function start() {
  HTMLDialogElement.prototype.showModal = function () { this.setAttribute("open", ""); };
  render(<ProjectProvider><QueryClientProvider client={new QueryClient()}><MemoryRouter><Routes>
    <Route path="/" element={<StartPage />} />
    <Route path="/manager" element={<ModManagerPage />} />
    <Route path="/save-editor" element={<SaveEditorPage />} />
    <Route path="/manager/install" element={<p>Installer destination</p>} />
    <Route path="/studio" element={<p>Studio destination</p>} />
  </Routes></MemoryRouter></QueryClientProvider></ProjectProvider>);
}
it("offers Manager, Studio and Save Editor and opens Studio", () => {
  start();
  expect(screen.getAllByRole("button")).toHaveLength(3);
  fireEvent.click(screen.getByRole("button", { name: "Modding Studio" }));
  expect(screen.getByText("Studio destination")).toBeTruthy();
});
it("opens the separate Save Editor with a file chooser and a route back", () => {
  start();
  fireEvent.click(screen.getByRole("button", { name: "Save Editor" }));
  expect(screen.getByRole("button", { name: "Choose file…" })).toBeTruthy();
  expect(screen.getByText(/extracted game database/)).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Back" }));
  expect(screen.getByRole("button", { name: "Modding Studio" })).toBeTruthy();
});
it("opens the manager setup and can return to the menu", () => {
  start();
  fireEvent.click(screen.getByRole("button", { name: "Mod Manager" }));
  expect(screen.getByRole("dialog")).toBeTruthy();
  expect(screen.getByLabelText("Downloaded mods folder")).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
  expect(screen.getByRole("button", { name: "Modding Studio" })).toBeTruthy();
});

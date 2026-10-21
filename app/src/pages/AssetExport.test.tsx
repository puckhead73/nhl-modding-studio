// @vitest-environment jsdom
import { render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, expect, it, vi } from "vitest";
import { AssetExportPage } from "./AssetExport";

const state = vi.hoisted(() => ({ players: [] as unknown[] }));
vi.mock("./PlayerAssets", () => ({
  usePlayerAssets: () => ({
    projectId: 7, installDir: "D:/NHL07",
    rootArgs: { install_dir: "D:/NHL07", platform: "xbox" },
    roots: { data: { complete: true } },
    assets: { data: state.players, isLoading: false },
  }),
  AssetGate: () => null,
  isExportable: (player: { slug?: string; mesh_path?: string }) => !!player.slug && !!player.mesh_path,
}));
vi.mock("../components/ProjectInputsCard", () => ({
  ProjectInputsCard: () => <div>Game source folders</div>,
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
vi.mock("../lib/client", () => ({ call: vi.fn() }));

beforeEach(() => { state.players = []; });

it("offers the full catalog even when no roster player has a named head", () => {
  render(<QueryClientProvider client={new QueryClient()}><AssetExportPage /></QueryClientProvider>);
  expect((screen.getByRole("button", { name: "Build appearance catalog…" }) as HTMLButtonElement).disabled).toBe(false);
  expect((screen.getByRole("button", { name: "Export 0 players…" }) as HTMLButtonElement).disabled).toBe(true);
  expect(screen.getByText("Head and hair textures (.dds)")).toBeTruthy();
  expect(screen.getByText("Game source folders")).toBeTruthy();
});

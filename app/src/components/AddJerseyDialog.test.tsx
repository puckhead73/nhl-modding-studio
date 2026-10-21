// @vitest-environment jsdom
//
// Tester feedback (20 Oct 2026): a packed jersey table used to be a dead end in "Add New Jersey".
// The dialog now offers a texture-files-only route — clone the kit's files under the new id, write
// no database row, and say so — so kits can be built here and registered in a database edited
// elsewhere. jsdom only.
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { TeamJerseyDto } from "../bindings/TeamJerseyDto";

type Pending = { cmd: string; args: any; resolve: (v: any) => void; reject: (e: any) => void };
const calls: Pending[] = [];
vi.mock("../lib/client", () => ({
  call: (cmd: string, args: any) =>
    new Promise((resolve, reject) => {
      calls.push({ cmd, args, resolve, reject });
    }),
}));
const update = vi.fn();
const roots = { install_dir: "H:/extracted", platform: "xbox", rendering_override: null, fe_override: null };
vi.mock("../lib/project", () => ({
  useProject: () => ({ update }),
  useAssetRootArgs: () => ({ installDir: "H:/extracted", rootArgs: roots }),
}));

import { AddJerseyDialog } from "./UniformsTab";

const source: TeamJerseyDto = {
  record: 40,
  jersey_index: 0,
  style_raw: 1,
  team_art_id: 131,
  variant_raw: 0,
  fe_variant_raw: 0,
  helmet_logo: 0,
  helmet_numbers: 0,
  is_light_color: 0,
  has_tie: 0,
  loc_string_raw: 0,
  is_default: true,
  is_lockable: false,
};

const FULL = "record allocation failed: the jersey table is full (703/703 rows) — this database has no headroom for a new kit";

function mount(installDir: string | null = "H:/extracted") {
  const onCreated = vi.fn();
  const onClose = vi.fn();
  render(
    <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
      <AddJerseyDialog
        projectId={7}
        teamId={3}
        source={source}
        kits={[{ key: "1_0", style: 1, variant: 0, jersey: source, textures: [] }]}
        rootArgs={roots as any}
        installDir={installDir}
        onClose={onClose}
        onCreated={onCreated}
      />
    </QueryClientProvider>,
  );
  return { onCreated, onClose };
}

beforeEach(() => {
  calls.length = 0;
  update.mockReset();
});
afterEach(cleanup);

describe("AddJerseyDialog — texture files only", () => {
  it("turns a full jersey table into the files-only route instead of a dead end", async () => {
    const { onCreated } = mount();
    fireEvent.click(screen.getByRole("button", { name: "Create jersey" }));
    await waitFor(() => expect(calls.map((c) => c.cmd)).toEqual(["clone_jersey"]));
    calls[0].reject(new Error(FULL));

    await screen.findByText("The jersey table in this database is full.");
    fireEvent.click(screen.getByRole("button", { name: "Create texture files only" }));

    // Files-only mode: the warning is explicit and the primary action changes.
    expect(screen.getByText(/will.*be added to the database/)).toBeTruthy();
    expect(screen.getByText("not")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Create texture files" }));
    await waitFor(() => expect(calls.map((c) => c.cmd)).toEqual(["clone_jersey", "clone_kit_textures"]));
    expect(calls[1].args).toMatchObject({ srcStyle: 1, srcArtId: 131, srcVariant: 0, dstStyle: 1, dstArtId: 131, dstVariant: 1 });
    calls[1].resolve({ files: [{ dst: "rendering/jersey_1_131_1_cm.rx2", rewrites: 2 }], skipped_existing: [] });

    await screen.findByText("Texture files for 1_131_1 created");
    expect(screen.getByText(/No database row was written/)).toBeTruthy();
    // No row → nothing to select, and the project summary (pending-edit count) is untouched.
    expect(onCreated).not.toHaveBeenCalled();
    expect(update).not.toHaveBeenCalled();
    expect(calls.map((c) => c.cmd)).toEqual(["clone_jersey", "clone_kit_textures"]);
  });

  it("can be chosen up front, and still refuses an id the containers cannot hold", async () => {
    mount();
    fireEvent.click(screen.getByLabelText(/Texture files only/));
    expect(screen.getByText(/will.*be added to the database/)).toBeTruthy();
    // A different-length id has no texture route at all.
    fireEvent.change(screen.getByLabelText("Variant (FccT)"), { target: { value: "12" } });
    expect((screen.getByRole("button", { name: "Create texture files" }) as HTMLButtonElement).disabled).toBe(true);
    fireEvent.change(screen.getByLabelText("Variant (FccT)"), { target: { value: "2" } });
    fireEvent.click(screen.getByRole("button", { name: "Create texture files" }));
    await waitFor(() => expect(calls.map((c) => c.cmd)).toEqual(["clone_kit_textures"]));
  });

  it("is unavailable without an install tree", () => {
    mount(null);
    expect((screen.getByLabelText(/Texture files only/) as HTMLInputElement).disabled).toBe(true);
  });
});

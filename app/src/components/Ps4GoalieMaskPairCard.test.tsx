// @vitest-environment jsdom
//
// Equipment 4.5 G3 (receipt 51) — the PS4 mask-pair card: a unique pair, a reported (not refused) helmet type 99,
// a catalogue that could not be read, and nothing for a non-PS4 goalie.
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, describe, expect, it, vi } from "vitest";

let dto: any = null;
vi.mock("../lib/client", () => ({
  call: (cmd: string) => (cmd === "ps4_gear_thumbnail" ? Promise.resolve({ width: 128, height: 128, png_data_url: "data:image/png;base64,AAAA" }) : Promise.resolve(dto)),
}));

import { Ps4GoalieMaskPairCard } from "./Ps4GoalieMaskPairCard";

const roots = { install_dir: "H:/CUSA15439", platform: "ps4", rendering_override: null, fe_override: null } as any;
function mount() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <Ps4GoalieMaskPairCard projectId={1} playerId={2} roots={roots} fieldsKey="125/0" />
    </QueryClientProvider>,
  );
}
const base = { applies: true, maskid: 125, helmet_type: 0, pack_debug_name: null, colour_map: null, catalogue: null, catalogue_problem: null, notes: ["not observed"] };

describe("Ps4GoalieMaskPairCard", () => {
  afterEach(cleanup);

  it("shows a unique pair with its details behind disclosure", async () => {
    dto = { ...base, class: "unique", text: "Mask 125 · helmet type 0: one catalogue mask", pack_debug_name: "goaliehelmet_itech_generic_8_texpack", colour_map: "rendering\characters\player\goaliehelmet_itech_generic_8_cm", catalogue: "nhl20 vault in databigs/boot.big" };
    mount();
    expect(await screen.findByText(/one catalogue mask/)).toBeTruthy();
    expect(screen.getByText("in the catalogue")).toBeTruthy();
    expect(screen.queryByText(/Reported, not refused/)).toBeNull();
    expect(screen.getByText("goaliehelmet_itech_generic_8_texpack")).toBeTruthy();
    expect(screen.queryByTestId("mask-open")).toBeNull();
  });

  it("opens the pair's picture and labels a name-convention candidate", async () => {
    dto = { ...base, class: "unique", text: "Mask 125 · helmet type 0: one catalogue mask", thumbnail_archive: "inxp_03.big", thumbnail_member: "nhl/ui/kettle_assets/creationzone/equipment/mask/mask125.gnf", thumbnail_basis: "name_convention_candidate", mask_name: "BAUER Yellow" };
    mount();
    fireEvent.click(await screen.findByTestId("mask-open"));
    expect(await screen.findByTestId("mask-thumb")).toBeTruthy();
    expect(screen.getByText(/a candidate, not a list row/)).toBeTruthy();
    expect(screen.getByTestId("mask-name").textContent).toBe("BAUER Yellow");
  });

  it("reports helmet type 99 without refusing", async () => {
    dto = { ...base, maskid: 0, helmet_type: 99, class: "not_a_catalogue_helmet_type", text: "stored helmet type 99; no catalogue helmet has that type" };
    mount();
    expect(await screen.findByText("helmet type not in the catalogue")).toBeTruthy();
    expect(screen.getByText(/Reported, not refused/)).toBeTruthy();
  });

  it("names why the catalogue was not read, and renders nothing when the check does not apply", async () => {
    dto = { ...base, class: null, text: "Mask 125 · helmet type 0", catalogue_problem: "source mismatch: the attached install's vault is nhl20's, but the open project is nhl19" };
    mount();
    expect(await screen.findByText(/source mismatch/)).toBeTruthy();
    cleanup();
    dto = { ...base, applies: false, class: null, text: "" };
    const { container } = mount();
    await new Promise((r) => setTimeout(r, 20));
    expect(container.textContent).toBe("");
  });
});

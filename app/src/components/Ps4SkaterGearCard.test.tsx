// @vitest-environment jsdom
//
// Receipt 52 — the PS4 skater gear card: list items with keys behind the name, helmet 99 reported, a thumbnail on
// demand, and a named problem when the install is not the title's own.
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, describe, expect, it, vi } from "vitest";

const calls: Array<{ cmd: string; args: any }> = [];
let dto: any = null;
vi.mock("../lib/client", () => ({
  call: (cmd: string, args: any) => {
    calls.push({ cmd, args });
    if (cmd === "ps4_gear_thumbnail") return Promise.resolve({ width: 164, height: 164, png_data_url: "data:image/png;base64,AAAA" });
    return Promise.resolve(dto);
  },
}));
import { Ps4SkaterGearCard } from "./Ps4SkaterGearCard";

const roots = { install_dir: "H:/CUSA15439", platform: "ps4", rendering_override: null, fe_override: null } as any;
function mount() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <Ps4SkaterGearCard projectId={1} playerId={2} roots={roots} fieldsKey="1/2/3/4/3" />
    </QueryClientProvider>,
  );
}

describe("Ps4SkaterGearCard", () => {
  afterEach(() => {
    cleanup();
    calls.length = 0;
  });

  it("lists items, reports helmet 99 and opens a thumbnail", async () => {
    dto = {
      applies: true,
      catalogue: "nhl20 lists: skaterhelmetstyle.csv bfb440e9aada…",
      problem: null,
      notes: ["not observed"],
      items: [
        { family: "helmet", label: "Helmet", value: 99, class: "outside", text: "stored 99; not in the catalogue", key: null, thumbnail_archive: null, thumbnail_member: null },
        { family: "visor", label: "Visor type", value: 3, class: "row", text: "visor 3: a Creation Zone item", key: "$TXT_VISOR_AVIATOR", name: "Aviator", thumbnail_archive: "inxp_03.big", thumbnail_member: "nhl/ui/kettle_assets/creationzone/equipment/visor/visor3.gnf" },
      ],
    };
    mount();
    expect(await screen.findByText("stored 99; not in the catalogue")).toBeTruthy();
    expect(screen.getByText("not in the catalogue")).toBeTruthy();
    // Receipt 57: the title's own text is shown; the key moves behind the tooltip.
    expect(screen.getByTestId("gear-name-visor").textContent).toBe("Aviator");
    expect(screen.getByTestId("gear-name-visor").getAttribute("title")).toContain("$TXT_VISOR_AVIATOR");
    expect(screen.queryByText("$TXT_VISOR_AVIATOR")).toBeNull();
    expect(screen.queryByTestId("gear-open-helmet")).toBeNull();
    fireEvent.click(screen.getByTestId("gear-open-visor"));
    expect(await screen.findByTestId("gear-thumb-visor")).toBeTruthy();
    expect(calls.find((c) => c.cmd === "ps4_gear_thumbnail")!.args).toMatchObject({ archive: "inxp_03.big", member: "nhl/ui/kettle_assets/creationzone/equipment/visor/visor3.gnf" });
  });

  it("names a source mismatch", async () => {
    dto = {
      applies: true,
      catalogue: null,
      problem: "source mismatch: the attached install's vault is nhl18's, but the open project is nhl17",
      notes: [],
      items: [{ family: "helmet", label: "Helmet", value: 1, class: null, text: "Helmet 1", key: null, thumbnail_archive: null, thumbnail_member: null }],
    };
    mount();
    expect(await screen.findByText(/source mismatch/)).toBeTruthy();
  });
});

// @vitest-environment jsdom
//
// P3.2-B (25 Sep 2026) — the Attributes tab's scale caption follows the offset the backend probed
// and stamped on the rating cells: NHL 11–Legacy "raw + 36 → 36..99", NHL 07–10 "raw + 50 → 50..99",
// PS4 "stored 0..99". The raw companion table id stays behind a tooltip, never in the caption text.
import { cleanup, render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, describe, expect, it, vi } from "vitest";

vi.mock("../lib/client", () => ({ call: () => Promise.reject(new Error("unmocked")) }));
vi.mock("../lib/project", () => ({
  useProject: () => ({
    project: { capabilities: { skater_ratings_labelled: 1, skater_ratings_total: 1, goalie_ratings_labelled: 1, goalie_ratings_total: 1 } },
  }),
}));

import { AttributesTab } from "./AttributesTab";

function player(offset: number): any {
  const field = {
    studio_table: "dSvy",
    studio_field: "abcd",
    name: offset === 0 ? "c_speed" : "p_speed",
    kind: "int",
    display: String(40 + offset),
    decoded: 40,
    enum_label: null,
    enum_options: [],
    edit_offset: offset,
    edit_height: false,
    edit_scale: null,
    raw_int: 40,
    text: null,
    range_low: 0,
    range_high: offset === 0 ? 99 : 63,
    editable: true,
    is_string: false,
  };
  return {
    player_id: 1,
    bio_record: 0,
    kind: "skater",
    ratings_record: 0,
    ratings_table: "dSvy",
    attributes: {
      kind: "skater",
      computed_overall: null,
      stored_overall: null,
      style: null,
      potential_range: null,
      potential_accuracy: null,
      flat: [{ label: "Speed", is_rating: true, field }],
      traits: [],
      trait_options: [],
      other_fields: [],
    },
  };
}

function mount(offset: number) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <AttributesTab projectId={1} player={player(offset)} />
    </QueryClientProvider>,
  );
}

afterEach(cleanup);

describe("AttributesTab scale caption", () => {
  it.each([
    [36, "raw + 36 → 36..99"],
    [50, "raw + 50 → 50..99"],
    [0, "stored 0..99"],
  ])("offset %i reads %s and keeps the table id behind a tooltip", (offset, text) => {
    mount(offset);
    const cap = screen.getByTestId("ratings-scale-caption");
    expect(cap.textContent).toContain(text);
    expect(cap.textContent).not.toContain("dSvy");
    expect(cap.getAttribute("title")).toContain("dSvy");
    expect(screen.getByText(String(40 + offset))).toBeTruthy();
  });
});

// @vitest-environment jsdom
//
// Title-scoped vocabularies (25 Sep 2026): an equipment field whose catalogue belongs to another
// title shows its stored number with a "no catalogue for this title" marker (the reason in the
// tooltip) and no dropdown; the same field on the catalogue's own title shows the label and the
// provenance in the tooltip; a field with no vocabulary at all shows neither.
import { cleanup, render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, describe, expect, it, vi } from "vitest";

vi.mock("../lib/client", () => ({
  call: () => Promise.reject(new Error("unmocked")),
}));
vi.mock("../lib/project", () => ({
  useProject: () => ({ update: () => {} }),
}));

import { FieldRow } from "./FieldRow";

const base: any = {
  studio_table: "xmja",
  studio_field: "wXFx",
  name: "stick",
  kind: "integer",
  decoded: 17,
  display: "17",
  enum_label: null,
  enum_options: [],
  edit_offset: null,
  edit_height: false,
  edit_scale: null,
  raw_int: 17,
  text: null,
  range_low: 0,
  range_high: 127,
  editable: true,
  is_string: false,
  text_max_bytes: null,
  vocabulary_note: null,
};

function mount(field: any) {
  const qc = new QueryClient();
  return render(
    <QueryClientProvider client={qc}>
      <FieldRow projectId={1} record={5} field={field} />
    </QueryClientProvider>,
  );
}

afterEach(cleanup);

describe("FieldRow title-scoped vocabularies", () => {
  it("shows the number and the no-catalogue marker on a title outside the catalogue's scope", () => {
    const note =
      "Stick Model: no catalogue for NHL 10 — its labels were established for NHL Legacy only; the stored id is shown as a number and written as entered";
    mount({ ...base, vocabulary_note: note });
    expect(screen.getByText("17")).toBeTruthy();
    const marker = screen.getByTestId("no-catalogue-for-title");
    expect(marker.textContent).toBe("no catalogue for this title");
    expect(marker.getAttribute("title")).toBe(note);
    expect(screen.queryByRole("combobox")).toBeNull();
    expect(screen.getByText("stick").getAttribute("title")).toContain("no catalogue for NHL 10");
  });

  it("shows the label with the provenance in the tooltip on the catalogue's own title", () => {
    const note = "labels established for NHL Legacy — texlib artwork (NHL Legacy PS3)";
    mount({
      ...base,
      display: "Easton Stealth S17",
      enum_label: "Easton Stealth S17",
      enum_options: [{ raw: 17, label: "Easton Stealth S17" }],
      vocabulary_note: note,
    });
    expect(screen.getByText("Easton Stealth S17")).toBeTruthy();
    expect(screen.queryByTestId("no-catalogue-for-title")).toBeNull();
    expect(screen.getByText("stick").getAttribute("title")).toContain("labels established for NHL Legacy");
  });

  it("shows neither marker nor provenance for a field without a vocabulary", () => {
    mount({ ...base, name: "stickcurve", studio_field: "MMuu", decoded: 2, display: "2", raw_int: 2 });
    expect(screen.queryByTestId("no-catalogue-for-title")).toBeNull();
    expect(screen.getByText("stickcurve").getAttribute("title")).toBe("MMuu");
  });
});

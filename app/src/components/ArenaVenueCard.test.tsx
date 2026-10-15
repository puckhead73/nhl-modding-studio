// @vitest-environment jsdom
//
// Phase 2.5 (22 Sep 2026) — the venue-parameter card: fields group by class with the class gloss,
// a colour zone renders as a swatch and its hex rather than a bare number, a field Studio will not
// edit is marked read-only and carries its reason, constants are hidden until asked for, and the
// "not a runtime effect" disclaimer is always on screen. jsdom only.
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const calls: Array<{ cmd: string; args: any }> = [];
let payload: any = null;
let fail: string | null = null;
vi.mock("../lib/client", () => ({
  call: (cmd: string, args: any) => {
    calls.push({ cmd, args });
    return fail ? Promise.reject(new Error(fail)) : Promise.resolve(payload);
  },
}));

vi.mock("../lib/project", () => ({
  useProject: () => ({ install: { install_dir: "H:/extracted" }, iso: null }),
}));

import { ArenaVenueCard } from "./ArenaVenueCard";

const view = () => ({
  arena_id: 12,
  record: 5,
  rows: 450,
  class_counts: [
    ["identity", 1],
    ["capacity", 2],
    ["unknown", 1],
  ],
  fields: [
    { name: "index", label: "Index", class: "identity", class_note: "the arena's own name, id and index", value: 12, colour: null, distinct: 450, constant: null, resolved: null, resolved_note: null, resolved_key: null, resolved_key_lookup: null, bank_file: null, editable: false, refusal: "edited on the arena's identity card, not here" },
    { name: "maxattendance", label: "Max attendance", class: "capacity", class_note: "attendance and crowd figures", value: 18500, colour: null, distinct: 40, constant: null, resolved: null, resolved_note: null, resolved_key: null, resolved_key_lookup: null, bank_file: null, editable: true, refusal: null },
    { name: "seat1zone1", label: "Seat 1 zone 1", class: "capacity", class_note: "attendance and crowd figures", value: 14671319, colour: "#DFDFD7", distinct: 12, constant: null, resolved: null, resolved_note: null, resolved_key: null, resolved_key_lookup: null, bank_file: null, editable: true, refusal: null },
    { name: "architectureid", label: "Architecture id", class: "unknown", class_note: "not classified — neither the name nor the value space suggests a meaning", value: 0, colour: null, distinct: 1, constant: 0, resolved: null, resolved_note: null, resolved_key: null, resolved_key_lookup: null, bank_file: null, editable: false, refusal: "this field is not classified: …" },
    { name: "goalhorn", label: "Goal horn", class: "horn", class_note: "goal / period horn selection and the horn-builder columns", value: 1, colour: null, distinct: 30, constant: null, resolved: "$NHLTeamName_ANA (AnaheimDucks)", resolved_note: "premade club horn: audioid 0 = value − 1 — team row; name key `$NHLTeamName_ANA` shown as the key: no dictionary is attached to this project", resolved_key: "$NHLTeamName_ANA", resolved_key_lookup: "no-dictionary", bank_file: null, editable: true, refusal: null },
    { name: "periodhorn", label: "Period horn", class: "horn", class_note: "goal / period horn selection and the horn-builder columns", value: 690, colour: null, distinct: 4, constant: null, resolved: null, resolved_note: "690 is the \"no horn selected\" sentinel — it names no catalogue row", resolved_key: null, resolved_key_lookup: null, bank_file: null, editable: true, refusal: null },
  ],
  notes: ["65 venue fields on this title; 33 are constant…", "the class is a candidate…, not a traced runtime effect: no arena field here has a known consumer"],
});

function mount(arenaId = 12) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <MemoryRouter>
      <QueryClientProvider client={qc}>
        <ArenaVenueCard projectId={7} arenaId={arenaId} />
      </QueryClientProvider>
    </MemoryRouter>,
  );
}

describe("ArenaVenueCard", () => {
  beforeEach(() => {
    calls.length = 0;
    payload = view();
    fail = null;
  });
  afterEach(cleanup);

  it("groups by class, renders a colour zone as a swatch and marks what it will not edit", async () => {
    mount();
    await waitFor(() => expect(screen.getByTestId("venue-group-capacity")).toBeTruthy());
    expect(calls[0]).toEqual({ cmd: "arena_venue_view", args: { projectId: 7, arenaId: 12, installRoot: "H:/extracted" } });

    // The class gloss is shown, so the card never asserts a meaning of its own.
    expect(screen.getByTestId("venue-group-capacity").textContent).toContain("attendance and crowd figures");

    // A colour zone shows its hex, not the raw number; the swatch carries the colour.
    const seat = screen.getByTestId("venue-field-seat1zone1");
    expect(seat.textContent).toContain("#DFDFD7");
    expect(seat.textContent).not.toContain("14671319");
    // jsdom normalises the inline hex to its rgb() form — #DFDFD7 = rgb(223, 223, 215).
    expect(seat.querySelector("span[style]")?.getAttribute("style")).toContain("rgb(223, 223, 215)");

    // An identity field is listed but read-only, with its reason on the row.
    expect(screen.getByTestId("venue-readonly-index")).toBeTruthy();
    expect(screen.getByTestId("venue-field-index").getAttribute("title")).toContain("identity card");

    // An editable field is not marked read-only.
    expect(screen.queryByTestId("venue-readonly-maxattendance")).toBeNull();

    // The disclaimer is always present.
    expect(document.body.textContent).toContain("not a traced runtime effect");
  });

  it("hides constant fields until asked, then shows them with their reason", async () => {
    mount();
    await waitFor(() => expect(screen.getByTestId("venue-group-capacity")).toBeTruthy());
    // `architectureid` is constant → hidden, and the button counts it.
    expect(screen.queryByTestId("venue-field-architectureid")).toBeNull();
    const button = screen.getByTestId("venue-show-constants");
    expect(button.textContent).toContain("1 constant field");
    fireEvent.click(button);
    await waitFor(() => expect(screen.getByTestId("venue-field-architectureid")).toBeTruthy());
    expect(screen.getByTestId("venue-readonly-architectureid")).toBeTruthy();
    expect(screen.getByTestId("venue-field-architectureid").getAttribute("title")).toContain("not classified");
  });

  it("shows the domain's refusal instead of an empty card when the inventory is unavailable", async () => {
    fail = "defs/arena/venue-fields.toml: not found";
    mount();
    await waitFor(() => expect(screen.getByTestId("venue-error").textContent).toContain("venue-fields.toml"));
  });

  it("names a catalogued horn and says when a value is not in the catalogue", async () => {
    mount();
    await waitFor(() => expect(screen.getByTestId("venue-group-horn")).toBeTruthy());
    // The club horn reads as the club, with the rule behind the tooltip rather than on the row.
    const horn = screen.getByTestId("venue-resolved-goalhorn");
    expect(horn.textContent).toContain("AnaheimDucks");
    expect(horn.getAttribute("title")).toContain("value − 1");
    // The key is the display text while no dictionary is attached, and the state says so.
    expect(horn.textContent).toContain("$NHLTeamName_ANA");
    expect(horn.getAttribute("data-key-lookup")).toBe("no-dictionary");
    expect(horn.getAttribute("title")).toContain("no dictionary is attached");
    // A value the catalogue cannot place says so, and carries the reason — it does not show a row.
    const period = screen.getByTestId("venue-unresolved-periodhorn");
    expect(period.textContent).toContain("not in the catalogue");
    expect(period.getAttribute("title")).toContain("sentinel");
    expect(screen.queryByTestId("venue-resolved-periodhorn")).toBeNull();
  });
  it("links a gen7 horn to its bank and leaves a chant id as a number with its note", async () => {
    payload = view();
    payload.fields = payload.fields.filter((f: any) => f.name !== "goalhorn");
    payload.fields.push(
      { name: "goalhorn", label: "Goal horn", class: "horn", class_note: "horns", value: 1, colour: null, distinct: 30, constant: null, resolved: "audio/sfx/001_goal.abk", resolved_note: "goal horn bank by exact file presence", resolved_key: null, resolved_key_lookup: null, bank_file: "001_goal.abk", editable: true, refusal: null },
      { name: "chantaudioid", label: "Chant audio", class: "audio", class_note: "audio ids", value: 61, colour: null, distinct: 40, constant: null, resolved: null, resolved_note: "a chant bank with the same number is shipped: `061_chant.abk` (loose). A number match, not a binding. … No consumer that selects a chant bank has been traced (research R97 §1.4). Listing only; no edit route", resolved_key: null, resolved_key_lookup: null, bank_file: null, editable: true, refusal: null },
    );
    mount();
    await waitFor(() => expect(screen.getByTestId("venue-bank-goalhorn")).toBeTruthy());
    expect(screen.getByTestId("venue-bank-goalhorn").getAttribute("href")).toBe("/app/database/leagues/commentary?bank=001_goal.abk");
    expect(screen.queryByTestId("venue-bank-chantaudioid")).toBeNull();
    const chant = screen.getByTestId("venue-unresolved-chantaudioid");
    expect(chant.textContent).toBe("number only — selection not traced");
    expect(chant.getAttribute("title")).toContain("No consumer that selects a chant bank has been traced");
  });
});

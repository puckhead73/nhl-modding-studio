// @vitest-environment jsdom
//
// Misc 5.4 (22 Sep 2026) — the song catalogue card: rows with title key / composer / genre /
// eligibility, the genre pills, the filter re-queries with the typed text, the unavailable
// reason, and the error state. jsdom only.
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
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

import { SongCatalogueCard } from "./SongCatalogueCard";

const listing = () => ({
  available: true,
  reason: null,
  source: "H:/extracted/…/songs.csv",
  rows_total: 422,
  composers: 135,
  genre_counts: [
    { genre: 11, key: "$TXT_ROCK", label: "Rock", lookup: "exact", rows: 200 },
    { genre: 4, key: "$TXT_DUBSTEP", label: null, lookup: "absent", rows: 40 },
  ],
  query: "",
  matching: 422,
  truncated: true,
  rows: [
    { index: 30, identifier: "AlienDance", artid: 53, audioid: 195704, composer: "Bob Bradley", genre: 4, genre_key: "$TXT_DUBSTEP", is_goal: true, is_intro: false, is_powerplay: true, is_win: true, name: "$TXT_SONG_ALIEN_DANCE", title: "Alien Dance", title_lookup: "exact", genre_label: null, genre_lookup: "absent" },
    { index: 31, identifier: "AllEyesOnMe", artid: 56, audioid: 212602, composer: "William Davies", genre: 11, genre_key: "$TXT_ROCK", is_goal: true, is_intro: null, is_powerplay: false, is_win: true, name: "$TXT_SONG_ALL_EYES_ON_ME", title: null, title_lookup: "absent", genre_label: "Rock", genre_lookup: "exact" },
  ],
  notes: ["selectable songs (catalogue) — not the team's song: …", "no sound bank carries the catalogue audioids …"],
  dictionary: { locale: "eng_us", sha256: "abc", rows: 30799 },
});

function mount() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <SongCatalogueCard projectId={5} />
    </QueryClientProvider>,
  );
}

describe("SongCatalogueCard", () => {
  beforeEach(() => {
    calls.length = 0;
    payload = listing();
    fail = null;
  });
  afterEach(cleanup);

  it("lists the catalogue rows with genres and eligibility, and re-queries on filter", async () => {
    mount();
    await waitFor(() => expect(screen.getByTestId("songs-card")).toBeTruthy());
    expect(calls[0]).toEqual({ cmd: "song_catalogue", args: { projectId: 5, installRoot: "H:/extracted", query: "", limit: 200 } });
    expect(document.body.textContent).toContain("422 songs from 135 composers");
    // A resolved title reads as text; the raw key and its lookup state stay behind the tooltip.
    const alien = screen.getByTestId("songs-title-AlienDance");
    expect(alien.textContent).toBe("Alien Dance");
    expect(alien.getAttribute("title")).toContain("$TXT_SONG_ALIEN_DANCE — resolved through this project's attached dictionary");
    // An unresolved key is shown as the key, explicitly, never guessed from the key's spelling.
    const eyes = screen.getByTestId("songs-title-AllEyesOnMe");
    expect(eyes.textContent).toBe("$TXT_SONG_ALL_EYES_ON_ME");
    expect(eyes.getAttribute("data-lookup")).toBe("absent");
    expect(eyes.getAttribute("title")).toContain("not in this project's attached dictionary");
    expect(screen.getByTestId("songs-row-AlienDance").textContent).toContain("Bob Bradley");
    expect(screen.getByTestId("songs-row-AlienDance").textContent).toContain("$TXT_DUBSTEP");
    expect(screen.getByTestId("songs-row-AllEyesOnMe").textContent).toContain("Rock");
    expect(screen.getByTestId("songs-row-AlienDance").textContent).toContain("goal · power play · win");
    expect(screen.getByTestId("songs-row-AllEyesOnMe").textContent).toContain("goal · win");
    expect(screen.getByTestId("songs-genre-11").textContent).toBe("Rock");
    expect(screen.getByTestId("songs-genre-4").textContent).toBe("$TXT_DUBSTEP");
    expect(screen.getByTestId("songs-genres").textContent).toContain("Rock 200");
    expect(screen.getByTestId("songs-dictionary").textContent).toContain("eng_us, 30799 rows");
    expect(screen.getByTestId("songs-matching").textContent).toContain("422 matching · showing the first 2");
    expect(screen.getByTestId("songs-notes").textContent).toContain("not the team's song");
    fireEvent.change(screen.getByTestId("songs-filter"), { target: { value: "bradley" } });
    await waitFor(() => expect(calls.length).toBe(2));
    expect(calls[1].args.query).toBe("bradley");
  });

  it("says when no dictionary is attached and keeps every title as its key", async () => {
    const l = listing();
    payload = {
      ...l,
      dictionary: null,
      rows: l.rows.map((r: any) => ({ ...r, title: null, title_lookup: "no-dictionary", genre_label: null, genre_lookup: "no-dictionary" })),
      genre_counts: l.genre_counts.map((g: any) => ({ ...g, label: null, lookup: "no-dictionary" })),
    };
    mount();
    await waitFor(() => expect(screen.getByTestId("songs-card")).toBeTruthy());
    expect(screen.getByTestId("songs-dictionary").textContent).toContain("No dictionary is attached");
    expect(screen.getByTestId("songs-title-AlienDance").textContent).toBe("$TXT_SONG_ALIEN_DANCE");
    expect(screen.getByTestId("songs-title-AlienDance").getAttribute("title")).toContain("no dictionary is attached");
    expect(screen.getByTestId("songs-genre-11").textContent).toBe("$TXT_ROCK");
  });

  it("shows the reason when the title has no catalogue, and the error as text", async () => {
    payload = { ...listing(), available: false, reason: "NHL 15 / 16 ship no song catalogue (research R54 §1)", rows: [], rows_total: 0 };
    mount();
    await waitFor(() => expect(screen.getByTestId("songs-unavailable").textContent).toContain("NHL 15 / 16"));
    expect(screen.queryByTestId("songs-card")).toBeNull();
    cleanup();
    fail = "the open project has no recorded title";
    mount();
    await waitFor(() => expect(screen.getByTestId("songs-error").textContent).toContain("no recorded title"));
  });
});

// @vitest-environment jsdom
import { cleanup, render, screen, fireEvent, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, expect, it, vi } from "vitest";
const invoke = vi.hoisted(() => vi.fn());
vi.mock("../lib/client", () => ({ call: invoke }));
vi.mock("../lib/project", () => ({ useProject: () => ({ install: { platform: "ps4", game_dir: "D:/NHL21", install_dir: "D:/NHL21/deep/db" }, iso: null }) }));
vi.mock("./TextureReplace", () => ({ TextureThumbnail: () => null, useTextureReplace: () => ({}) }));
vi.mock("./ArchiveMemberReplace", () => ({ useArchiveMemberReplace: () => ({}) }));
import { LeagueLogoCard } from "./LeagueLogoCard";
import { TeamLogoCard } from "./TeamLogoCard";
afterEach(() => { cleanup(); invoke.mockReset(); });
it.each(["league", "team"])("%s logos use the PS4 game root and retain an actionable error with retry", async (kind) => {
 invoke.mockRejectedValue(new Error("The logo archive is missing. Relink the game folder."));
 const client = new QueryClient({defaultOptions:{queries:{retry:false}}});
 render(<QueryClientProvider client={client}>{kind === "league" ? <LeagueLogoCard projectId={1} leagueCode={0} /> : <TeamLogoCard projectId={1} teamId={0} />}</QueryClientProvider>);
 expect((await screen.findByRole("alert")).textContent).toContain("Relink the game folder");
 expect(invoke.mock.calls[0][1].installRoot).toBe("D:/NHL21");
 fireEvent.click(screen.getByRole("button", {name:"Retry"}));
 await waitFor(() => expect(invoke).toHaveBeenCalledTimes(2));
});

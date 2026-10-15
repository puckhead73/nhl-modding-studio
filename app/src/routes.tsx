import { createBrowserRouter, Navigate } from "react-router-dom";
import { AppShell } from "./AppShell";
import { OpenProject } from "./pages/OpenProject";
import { FrostbiteBrowser } from "./pages/FrostbiteBrowser";
import { StartPage } from "./pages/Start";
import { SaveEditorPage } from "./pages/SaveEditor";
import { ModManagerPage } from "./pages/ModManager";
import { InstallPackagePage } from "./pages/InstallPackage";
import { PlayersPage } from "./pages/Players";
import { PlayerDetail } from "./pages/PlayerDetail";
import { TeamsPage } from "./pages/Teams";
import { LeaguesPage } from "./pages/Leagues";
import { LeagueDetail } from "./pages/LeagueDetail";
import { LeagueImageryPage } from "./pages/LeagueImagery";
import { SpeechCataloguePage } from "./pages/SpeechCatalogue";
import { TeamDetail } from "./pages/TeamDetail";
import { ArenasPage } from "./pages/Arenas";
import { ArenaDetail } from "./pages/ArenaDetail";
import { Dashboard } from "./pages/Dashboard";
import { TransactionsPage } from "./pages/Transactions";
import { PendingEdits } from "./pages/PendingEdits";
import { SettingsGeneralPage, SettingsPathsPage } from "./pages/Settings";
import { ComingSoon } from "./pages/ComingSoon";
import { CorpusGate } from "./components/CorpusGate";
import { HistoricalPlayersPage } from "./pages/HistoricalPlayers";
import { HistoricalPlayerDetail } from "./pages/HistoricalPlayerDetail";
import { HistoricalTeamsPage } from "./pages/HistoricalTeams";
import { HistoricalTeamDetail } from "./pages/HistoricalTeamDetail";
import { RosterBuilderPage } from "./pages/RosterBuilder";
import { LegendsPicksPage } from "./pages/LegendsPicks";
import { LegendsTeamDetail } from "./pages/LegendsTeamDetail";
import { FormulasPage } from "./pages/Formulas";
import { PlayerAssetsIndex, PlayerAssetsPage } from "./pages/PlayerAssets";
import { PlayerAssetDetail } from "./pages/PlayerAssetDetail";
import { AssetExportPage } from "./pages/AssetExport";
import { AppearanceStudioPage } from "./pages/AppearanceStudio";
import { FileBrowserPage } from "./pages/FileBrowser";
import { TeamOutfitsPage } from "./pages/TeamOutfits";
import { HockeyShopPage } from "./pages/HockeyShop";
import { GameSettingsPage } from "./pages/GameSettings";
import { SelectPrompt } from "./components/primitives";
import { AboutPage, SettingsAboutPage } from "./pages/About";

export const router = createBrowserRouter([
  { path: "/", element: <StartPage /> },
  { path: "/studio", element: <OpenProject /> },
  { path: "/frostbite", element: <FrostbiteBrowser /> },
  { path: "/manager", element: <ModManagerPage /> },
  { path: "/save-editor", element: <SaveEditorPage /> },
  { path: "/manager/install", element: <InstallPackagePage /> },
  // About & licences (receipt 46) — reachable before a project is open.
  { path: "/about", element: <AboutPage /> },
  // ONB-003: the built-in package installer — reachable without a project.
  { path: "/install", element: <Navigate to="/manager/install" replace /> },
  {
    path: "/app",
    element: <AppShell />,
    children: [
      // The app lands on the Game Database; the on-load audit routes to the Dashboard on errors.
      { index: true, element: <Navigate to="database" replace /> },

      // Back-compat: the Dashboard is now a Settings subtab.
      { path: "dashboard", element: <Navigate to="/app/settings/dashboard" replace /> },

      // Database section — the EA-DB editing surfaces.
      {
        path: "database",
        children: [
          { index: true, element: <Navigate to="teams" replace /> },
          // Back-compat: the Dashboard used to live under the Database section.
          { path: "dashboard", element: <Navigate to="/app/settings/dashboard" replace /> },
          {
            path: "players",
            element: <PlayersPage />,
            children: [
              { index: true, element: <SelectPrompt what="player" /> },
              { path: ":id", element: <PlayerDetail /> },
            ],
          },
          {
            path: "leagues",
            element: <LeaguesPage />,
            children: [
              { index: true, element: <SelectPrompt what="league" /> },
              { path: "imagery", element: <LeagueImageryPage /> },
              { path: "commentary", element: <SpeechCataloguePage /> },
              { path: ":id", element: <LeagueDetail /> },
            ],
          },
          {
            path: "teams",
            element: <TeamsPage />,
            children: [
              { index: true, element: <SelectPrompt what="team" /> },
              { path: ":id", element: <TeamDetail /> },
            ],
          },
          // Back-compat: Lines is now a subtab of the Team detail page.
          { path: "lines", element: <Navigate to="/app/database/teams" replace /> },
          {
            path: "arenas",
            element: <ArenasPage />,
            children: [
              { index: true, element: <SelectPrompt what="arena" /> },
              { path: ":id", element: <ArenaDetail /> },
            ],
          },
          { path: "transactions", element: <TransactionsPage /> },
          { path: "game-settings", element: <GameSettingsPage /> },
          { path: "pending", element: <PendingEdits /> },
        ],
      },

      // Historical section — the FHM corpus browse (read-only). Mirrors the Game-Database master-
      // detail shape; gated on an open corpus data pack. Formulas + Roster Builder arrive later.
      {
        path: "historical",
        element: <CorpusGate />,
        children: [
          { index: true, element: <Navigate to="players" replace /> },
          {
            path: "players",
            element: <HistoricalPlayersPage />,
            children: [
              { index: true, element: <SelectPrompt what="player" /> },
              { path: ":id", element: <HistoricalPlayerDetail /> },
            ],
          },
          {
            path: "teams",
            element: <HistoricalTeamsPage />,
            children: [
              { index: true, element: <SelectPrompt what="team" /> },
              { path: ":id", element: <HistoricalTeamDetail /> },
            ],
          },
          { path: "builder", element: <RosterBuilderPage /> },
          { path: "formulas", element: <FormulasPage /> },
        ],
      },

      // Legends section — the team builder. Reads the historical corpus (so it sits behind the same
      // gate), but builds new teams rather than browsing recorded ones.
      {
        path: "legends",
        element: <CorpusGate />,
        children: [
          { index: true, element: <Navigate to="picks" replace /> },
          {
            path: "picks",
            element: <LegendsPicksPage />,
            children: [
              { index: true, element: <SelectPrompt what="franchise" /> },
              { path: ":id", element: <LegendsTeamDetail /> },
            ],
          },
        ],
      },

      // Hockey Shop — per-equipment-slot browse + 3D preview (store + install later).
      { path: "shop", element: <HockeyShopPage /> },

      // Assets section — the shipped art tools. Player portraits/heads land first (asset prep);
      // textures, logos, equipment and audio follow in Phase 6.
      {
        path: "assets",
        children: [
          { index: true, element: <Navigate to="players" replace /> },
          {
            path: "players",
            element: <PlayerAssetsPage />,
            children: [
              { index: true, element: <PlayerAssetsIndex /> },
              { path: ":id", element: <PlayerAssetDetail /> },
            ],
          },
          { path: "export", element: <AssetExportPage /> },
          { path: "appearance", element: <AppearanceStudioPage /> },
          { path: "outfits", element: <TeamOutfitsPage /> },
          { path: "files", element: <FileBrowserPage /> },
          {
            path: "textures",
            element: (
              <ComingSoon
                title="Texture Lab"
                hint="Browse, preview and replace the install's textures, plus team logos, equipment and audio. Arriving in a later phase."
              />
            ),
          },
        ],
      },
      {
        path: "settings",
        children: [
          { index: true, element: <Navigate to="general" replace /> },
          { path: "general", element: <SettingsGeneralPage /> },
          { path: "paths", element: <SettingsPathsPage /> },
          // Project overview + validation audit — reachable directly, and auto-opened on errors.
          { path: "dashboard", element: <Dashboard /> },
          { path: "about", element: <SettingsAboutPage /> },
        ],
      },

      // Back-compat: old flat URLs redirect into the Database section.
      { path: "players/*", element: <Navigate to="/app/database/players" replace /> },
      { path: "teams/*", element: <Navigate to="/app/database/teams" replace /> },
      { path: "lines", element: <Navigate to="/app/database/lines" replace /> },
      { path: "arenas/*", element: <Navigate to="/app/database/arenas" replace /> },
      { path: "pending", element: <Navigate to="/app/database/pending" replace /> },
    ],
  },
]);

import { createBrowserRouter, Navigate } from "react-router-dom";
import { AppShell } from "./AppShell";
import { OpenProject } from "./pages/OpenProject";
import { PlayersPage } from "./pages/Players";
import { PlayerDetail } from "./pages/PlayerDetail";
import { TeamsPage } from "./pages/Teams";
import { TeamDetail } from "./pages/TeamDetail";
import { ArenasPage } from "./pages/Arenas";
import { ArenaDetail } from "./pages/ArenaDetail";
import { Dashboard } from "./pages/Dashboard";
import { TransactionsPage } from "./pages/Transactions";
import { PendingEdits } from "./pages/PendingEdits";
import { SettingsPage } from "./pages/Settings";
import { ComingSoon } from "./pages/ComingSoon";
import { CorpusGate } from "./components/CorpusGate";
import { HistoricalPlayersPage } from "./pages/HistoricalPlayers";
import { HistoricalPlayerDetail } from "./pages/HistoricalPlayerDetail";
import { HistoricalTeamsPage } from "./pages/HistoricalTeams";
import { HistoricalTeamDetail } from "./pages/HistoricalTeamDetail";
import { FormulasPage } from "./pages/Formulas";
import { SelectPrompt } from "./components/primitives";

export const router = createBrowserRouter([
  { path: "/", element: <OpenProject /> },
  {
    path: "/app",
    element: <AppShell />,
    children: [
      { index: true, element: <Navigate to="dashboard" replace /> },

      // Dashboard — its own top-level section (project overview + validation).
      { path: "dashboard", element: <Dashboard /> },

      // Database section — the EA-DB editing surfaces.
      {
        path: "database",
        children: [
          { index: true, element: <Navigate to="teams" replace /> },
          // Back-compat: the Dashboard used to live under the Database section.
          { path: "dashboard", element: <Navigate to="/app/dashboard" replace /> },
          {
            path: "players",
            element: <PlayersPage />,
            children: [
              { index: true, element: <SelectPrompt what="player" /> },
              { path: ":id", element: <PlayerDetail /> },
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
          { path: "formulas", element: <FormulasPage /> },
        ],
      },

      // Sections built out in later phases — honest placeholders for now.
      {
        path: "assets",
        element: (
          <ComingSoon
            title="Production Asset Tools"
            hint="Edit textures, portraits/logos, equipment, appearance, localization strings, and audio. Arriving in a later phase."
          />
        ),
      },
      {
        path: "builds",
        element: (
          <ComingSoon
            title="Builds"
            hint="Create, switch, and manage named snapshots of your install + save state. Arriving in a later phase."
          />
        ),
      },

      { path: "settings", element: <SettingsPage /> },

      // Back-compat: old flat URLs redirect into the Database section.
      { path: "players/*", element: <Navigate to="/app/database/players" replace /> },
      { path: "teams/*", element: <Navigate to="/app/database/teams" replace /> },
      { path: "lines", element: <Navigate to="/app/database/lines" replace /> },
      { path: "arenas/*", element: <Navigate to="/app/database/arenas" replace /> },
      { path: "pending", element: <Navigate to="/app/database/pending" replace /> },
    ],
  },
]);

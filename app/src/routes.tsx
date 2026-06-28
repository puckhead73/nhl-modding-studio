import { createBrowserRouter, Navigate } from "react-router-dom";
import { AppShell } from "./AppShell";
import { OpenProject } from "./pages/OpenProject";
import { PlayersPage } from "./pages/Players";
import { PlayerDetail } from "./pages/PlayerDetail";
import { TeamsPage } from "./pages/Teams";
import { TeamDetail } from "./pages/TeamDetail";
import { ArenasPage } from "./pages/Arenas";
import { ArenaDetail } from "./pages/ArenaDetail";

export const router = createBrowserRouter([
  { path: "/", element: <OpenProject /> },
  {
    path: "/app",
    element: <AppShell />,
    children: [
      { index: true, element: <Navigate to="players" replace /> },
      {
        path: "players",
        element: <PlayersPage />,
        children: [{ path: ":id", element: <PlayerDetail /> }],
      },
      {
        path: "teams",
        element: <TeamsPage />,
        children: [{ path: ":id", element: <TeamDetail /> }],
      },
      {
        path: "arenas",
        element: <ArenasPage />,
        children: [{ path: ":id", element: <ArenaDetail /> }],
      },
    ],
  },
]);

import { createBrowserRouter, Navigate } from "react-router-dom";
import { AppShell } from "./AppShell";
import { OpenProject } from "./pages/OpenProject";
import { PlayersPage } from "./pages/Players";
import { PlayerDetail } from "./pages/PlayerDetail";

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
    ],
  },
]);

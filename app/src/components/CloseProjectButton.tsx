import { useCallback } from "react";
import { useNavigate } from "react-router-dom";
import { useProject } from "../lib/project";
import { useAcknowledgedClose } from "../lib/closeProject";

/// The shell's Close: releases the server-side workspace first and leaves the project only once
/// the server acknowledged (`closed` / `already_closed`). A refusal — a copied-target save still
/// running, a review that could not be discarded — or an IPC failure keeps the project open in
/// the shell with the reason beside the button, so the running operation's progress and Cancel
/// stay reachable and the user can retry (LEAGUE-SMOKE-R1).
export function CloseProjectButton({ projectId }: { projectId: number }) {
  const { close } = useProject();
  const navigate = useNavigate();
  const onClosed = useCallback(() => {
    close();
    navigate("/", { replace: true });
  }, [close, navigate]);
  const { pending, error, requestClose } = useAcknowledgedClose(projectId, onClosed);
  return (
    <span className="relative flex items-center">
      <button
        type="button"
        onClick={() => void requestClose()}
        disabled={pending}
        aria-busy={pending}
        className={
          "rounded-md border px-2.5 py-1 text-xs hover:bg-slate-800 disabled:cursor-not-allowed disabled:opacity-50 " +
          (error ? "border-rose-700 text-rose-200" : "border-slate-700 text-slate-300")
        }
      >
        {pending ? "Closing…" : error ? "Retry close" : "Close"}
      </button>
      {/* The reason sits in a popover under the button, so a long refusal never spills across the
          header's tabs; the shell (and the running operation's controls) stay exactly as they were. */}
      {error && (
        <span
          role="alert"
          className="absolute right-0 top-full z-20 mt-1 w-96 rounded-md border border-rose-800 bg-slate-950 p-2 text-left text-xs text-rose-200 shadow-lg"
        >
          {error}
        </span>
      )}
    </span>
  );
}

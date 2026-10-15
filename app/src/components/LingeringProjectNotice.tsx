import { useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { useProject } from "../lib/project";
import { useAcknowledgedClose } from "../lib/closeProject";
import { Button } from "./primitives";

/// The picker's corruption-escape close (LEAGUE-SMOKE-R1). The CorruptionGuard sends the user
/// here with the damaged project still open and leaves the close to the picker (closing inside the
/// shell would trigger its own redirect). This notice asks the server to release that project
/// once on mount and clears the shell's owner only when the server acknowledged. A refusal (a
/// copied-target operation still holds it) or a transport failure keeps the owner: the notice
/// names the reason, offers **Retry**, and offers **Return to project** — a plain navigation back
/// into the still-open shell, never a re-open — so a live operation is never orphaned. While the
/// project lingers the picker's *Open project* is held back by the parent (`blocked`), so a new
/// session cannot silently replace the one that failed to close. No redirect happens here: the
/// picker route stays put whatever the outcome.
export function LingeringProjectNotice() {
  const { project, close } = useProject();
  const navigate = useNavigate();
  const projectId = project?.project_id ?? null;
  const { pending, error, requestClose } = useAcknowledgedClose(projectId, close);
  // One request on mount for the project that is lingering; retries are explicit.
  useEffect(() => {
    if (projectId != null) void requestClose();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  if (projectId == null) return null;
  return (
    <div
      role={error ? "alert" : "status"}
      className={
        "mb-5 rounded-md border p-3 text-xs " +
        (error ? "border-rose-800 bg-rose-950/60 text-rose-100" : "border-slate-700 bg-slate-900/60 text-slate-300")
      }
    >
      {error ? (
        <>
          <div className="text-sm font-semibold">The previous project is still open</div>
          <p className="mt-1 text-rose-200/90">{error}</p>
          <div className="mt-2 flex gap-2">
            <Button variant="secondary" onClick={() => void requestClose()} disabled={pending}>
              Retry close
            </Button>
            <Button variant="secondary" onClick={() => navigate("/app/database/pending")}>
              Return to project
            </Button>
          </div>
        </>
      ) : (
        <span>{pending ? "Closing the previous project…" : "Closing the previous project."}</span>
      )}
    </div>
  );
}

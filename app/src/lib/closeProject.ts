// Acknowledged project close (LEAGUE-SMOKE-R1).
//
// The shell used to fire `close_project` and drop its owner at once. The server can refuse: a
// copied-target apply / restore / recovery still holds the session (`busy`), or a prepared review
// could not be discarded (`failed`) — and an IPC failure rejects outright. Dropping the owner in
// those cases hid a still-running save and its Cancel button behind the picker. This hook makes
// the transition wait for the server's answer: the caller's `onClosed` runs only for `closed` /
// `already_closed`; anything else keeps the owner and exposes the reason for a retry.
//
// Ownership: a request is bound to the project id it was made for and to a request generation.
// A response that arrives after the project changed, or after a newer request started, is
// ignored (never applied to the wrong session). One request is in flight at a time — a repeated
// click while pending is a no-op.
import { useCallback, useEffect, useRef, useState } from "react";
import { call } from "./client";
import { awaitFieldWrites } from "./fieldWrites";
import type { CloseProjectDto } from "../bindings/CloseProjectDto";

export type CloseState = {
  /** A close request is in flight for `projectId`. */
  pending: boolean;
  /** The last refusal or transport failure, for the user; cleared when a request starts. */
  error: string | null;
  /** The server's last outcome word (`busy`, `failed`, `closed`, `already_closed`) or `null`. */
  outcome: string | null;
};

const IDLE: CloseState = { pending: false, error: null, outcome: null };

/** Plain-language text for a refused close. */
export function describeRefusal(r: CloseProjectDto): string {
  const why = r.reason ? ` ${r.reason}` : "";
  return r.outcome === "busy"
    ? `The project is still in use by a running operation, so it was not closed.${why}`
    : `The project could not be closed.${why}`;
}

export function useAcknowledgedClose(projectId: number | null, onClosed: (projectId: number) => void) {
  const [state, setState] = useState<CloseState>(IDLE);
  const projectRef = useRef(projectId);
  const genRef = useRef(0);
  const mountedRef = useRef(true);
  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);
  // A project change invalidates any in-flight request and its displayed state.
  useEffect(() => {
    if (projectRef.current !== projectId) {
      projectRef.current = projectId;
      genRef.current += 1;
      setState(IDLE);
    }
  }, [projectId]);

  // Ref, not state: two clicks in one tick must not both pass the guard.
  const inFlightRef = useRef(false);
  const requestClose = useCallback(async () => {
    const id = projectRef.current;
    if (id == null || inFlightRef.current) return;
    inFlightRef.current = true;
    const gen = ++genRef.current;
    setState({ pending: true, error: null, outcome: null });
    const live = () => mountedRef.current && gen === genRef.current && projectRef.current === id;
    let result: CloseProjectDto;
    try {
      await awaitFieldWrites(id);
      result = await call("close_project", { projectId: id });
    } catch (e) {
      inFlightRef.current = false;
      // Transport failure: the server state is unknown, so the owner stays and the user retries.
      if (live()) setState({ pending: false, error: `Close did not complete: ${String(e)}. The project is still open here — try again.`, outcome: null });
      return;
    }
    inFlightRef.current = false;
    if (!live()) return;
    if (result.outcome === "closed" || result.outcome === "already_closed") {
      setState({ pending: false, error: null, outcome: result.outcome });
      onClosed(id);
      return;
    }
    setState({ pending: false, error: describeRefusal(result), outcome: result.outcome });
  }, [onClosed]);

  return { ...state, requestClose };
}

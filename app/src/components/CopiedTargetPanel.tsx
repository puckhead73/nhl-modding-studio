import { useCallback, useEffect, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { Button, Card, Pill } from "../components/primitives";
import type { CopiedTargetReviewDto } from "../bindings/CopiedTargetReviewDto";
import type { CopiedTargetApplyDto } from "../bindings/CopiedTargetApplyDto";
import type { CopiedTargetStatusDto } from "../bindings/CopiedTargetStatusDto";
import type { CopiedTargetRestoreDto } from "../bindings/CopiedTargetRestoreDto";
import type { ProjectBaselineDto } from "../bindings/ProjectBaselineDto";

/// The destination/archive configuration a review is minted for. A prepared review is bound to
/// the whole of it (plus the project and the server's workspace revision); the controls are
/// frozen while a review exists, and a review whose binding no longer matches is stale.
export type CopiedTargetConfig = {
  root: string;
  withArchive: boolean;
  archiveRel: string;
  memberPath: string;
  /// LOC-W1: also replace the dictionary member (the pending rename's output) inside an archive.
  withDictionaryMember: boolean;
  dictionaryArchiveRel: string;
  dictionaryMemberPath: string;
};

type BoundReview = {
  review: CopiedTargetReviewDto;
  projectId: number;
  /// `ProjectSummaryDto.workspace_revision` at review time (the server's content revision, the
  /// same value the handle is bound to — not an undo-depth heuristic).
  revision: string;
  config: CopiedTargetConfig;
};

const PS4_ARCHIVE = "databigs/inxp_00.big";
const PS4_MEMBER = "nhl/data/common/gamemodes/database/t3db/nhl_game.db";
const PS4_DICT_MEMBER = "nhl/data/kettle/loc/eng_us.db";
const X360_ARCHIVE = "cacheboot.big";
const X360_DICT_MEMBER = "fe/loc/nhl_eng_us.db";

function sameConfig(a: CopiedTargetConfig, b: CopiedTargetConfig): boolean {
  return (
    a.root === b.root &&
    a.withArchive === b.withArchive &&
    (!a.withArchive || (a.archiveRel === b.archiveRel && a.memberPath === b.memberPath)) &&
    a.withDictionaryMember === b.withDictionaryMember &&
    (!a.withDictionaryMember ||
      (a.dictionaryArchiveRel === b.dictionaryArchiveRel && a.dictionaryMemberPath === b.dictionaryMemberPath))
  );
}

/// Copied-target save: review → apply through the coordinated transaction engine, with status,
/// recovery and exact restore. The panel only ever talks to a **marked copied root** — a directory
/// the engine created and stamped — never to an installed game tree, and it never claims an
/// in-game installation: every apply lands in the copied tree as an unqualified artifact set.
///
/// Handle lifecycle (LEAGUE-R1 F3): a review mints a server-owned handle bound to the project, the
/// server workspace revision and the full destination/archive configuration. While a review is
/// prepared the configuration controls are frozen; the review is discarded on re-review, panel
/// unmount and project change; a review that arrives after the panel moved on (unmount, project
/// switch) is discarded immediately by the request generation it was started under; a review
/// whose revision no longer matches the session is stale (Apply disabled, backend refusal also
/// guards it). Handles the panel could not dispose of remain visible through the server's own
/// status (pending handles) with a Discard action — the durable owner path.
export function CopiedTargetPanel() {
  const { project, update } = useProject();
  const qc = useQueryClient();
  const p = project!;
  const projectId = p.project_id;
  const isPs4 = p.platform === "ps4";

  // ---- ownership guards ------------------------------------------------------------------
  // The request generation: bumped on project change and unmount. Every async completion checks
  // the generation it was started under; a mismatch means the result has no owner any more.
  const genRef = useRef(0);
  const projectIdRef = useRef(projectId);
  const mountedRef = useRef(true);
  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const [bound, setBound] = useState<BoundReview | null>(null);
  const boundRef = useRef<BoundReview | null>(null);
  boundRef.current = bound;

  const [config, setConfig] = useState<CopiedTargetConfig>({
    root: "",
    withArchive: false,
    archiveRel: PS4_ARCHIVE,
    memberPath: PS4_MEMBER,
    withDictionaryMember: false,
    dictionaryArchiveRel: isPs4 ? PS4_ARCHIVE : X360_ARCHIVE,
    dictionaryMemberPath: isPs4 ? PS4_DICT_MEMBER : X360_DICT_MEMBER,
  });
  const [outcome, setOutcome] = useState<CopiedTargetApplyDto | null>(null);
  const [restoreOutcome, setRestoreOutcome] = useState<CopiedTargetRestoreDto | null>(null);
  const [baseline, setBaseline] = useState<ProjectBaselineDto | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [cleanupFailures, setCleanupFailures] = useState<string[]>([]);

  /// Discard a handle the panel owns or a late one it no longer owns. A failure is recorded in
  /// the panel (when mounted) and remains visible through the server status's pending handles.
  const disposeHandle = useCallback(async (handle: number, why: string) => {
    try {
      await call("copied_target_discard", { handle });
    } catch (e) {
      const note = `Review handle ${handle} (${why}) could not be discarded: ${String(e)}. It stays listed under the copied root's status until discarded there.`;
      if (mountedRef.current) setCleanupFailures((prev) => [...prev, note]);
    }
  }, []);

  const discardCurrent = useCallback(
    async (why: string) => {
      const b = boundRef.current;
      if (!b) return;
      boundRef.current = null;
      if (mountedRef.current) setBound(null);
      await disposeHandle(b.review.handle, why);
    },
    [disposeHandle],
  );

  // Project change: the previous project's review has no owner any more; bump the generation so
  // in-flight requests started for the old project dispose of their result.
  useEffect(() => {
    if (projectIdRef.current !== projectId) {
      genRef.current += 1;
      void discardCurrent("project changed");
      setOutcome(null);
      setRestoreOutcome(null);
      setBaseline(null);
      setMessage(null);
      projectIdRef.current = projectId;
    }
  }, [projectId, discardCurrent]);

  // Unmount: bump the generation (late results dispose of themselves) and discard what is held.
  useEffect(() => {
    return () => {
      genRef.current += 1;
      void discardCurrent("panel closed");
    };
  }, [discardCurrent]);

  /// Refresh queries and the project summary — but never let an old project's summary overwrite
  /// the one the shell now shows. The query invalidation is not awaited: every active query
  /// refetches serially behind the same session lock (seconds in a debug build), and the caller's
  /// outcome or error must not wait behind them (LEAGUE-RESP-1).
  const refresh = useCallback(
    async (forProject: number) => {
      void qc.invalidateQueries();
      const summary = await call("project_summary", { projectId: forProject });
      if (mountedRef.current && summary.project_id === projectIdRef.current) update(summary);
    },
    [qc, update],
  );

  /// The apply consumed the handle whatever it returned (committed, refused, cancelled). Drop
  /// the panel's ownership only once the root status has been re-read: the cached status still
  /// lists the handle as prepared, and showing it as a foreign "review pending" with a Discard
  /// button in between — for as long as a refetch takes — is exactly the stale status the
  /// desktop measurement caught once cancellation settled in under a second (LEAGUE-RESP-1).
  /// `boundRef` is cleared first so a late unmount does not try to discard the consumed handle.
  const settleAppliedHandle = useCallback(
    async (root: string) => {
      boundRef.current = null;
      try {
        await qc.refetchQueries({ queryKey: ["copied-status", root] });
      } catch {
        // the status query keeps its own error; ownership is dropped either way
      }
      if (mountedRef.current) setBound(null);
    },
    [qc],
  );

  // ---- queries -----------------------------------------------------------------------------
  const rootLookup = useQuery({
    queryKey: ["copied-root", projectId],
    queryFn: () => call("copied_target_root_for", { projectId }),
  });
  useEffect(() => {
    if (rootLookup.data?.root && config.root === "" && !bound) {
      setConfig((c) => ({ ...c, root: rootLookup.data!.root! }));
    }
  }, [rootLookup.data, config.root, bound]);

  const status = useQuery({
    queryKey: ["copied-status", config.root],
    queryFn: () => call("copied_target_status", { root: config.root }),
    enabled: config.root.trim() !== "",
    retry: false,
  });

  // ---- derived binding state ---------------------------------------------------------------
  const staleReason: string | null = (() => {
    if (!bound) return null;
    if (bound.projectId !== projectId) return "the project changed";
    if (bound.revision !== p.workspace_revision) return "the session changed after the review (an edit, undo, redo or save)";
    if (!sameConfig(bound.config, config)) return "the destination or archive configuration changed";
    return null;
  })();
  const stale = staleReason != null;
  const frozen = bound != null;

  // ---- mutations ---------------------------------------------------------------------------
  const createRoot = useMutation({
    mutationFn: (path: string) => call("copied_target_create_root", { path }),
    onSuccess: (created) => {
      setConfig((c) => ({ ...c, root: created }));
      setMessage(`Created the copied-target root ${created}. Copy the game files into it and open them from there before saving.`);
    },
    // A refused creation must not leave the previous "Created …" note standing beside its error.
    onError: () => setMessage(null),
  });

  const doReview = useMutation({
    mutationFn: async () => {
      const gen = genRef.current;
      const forProject = projectId;
      const cfg = { ...config };
      const revision = p.workspace_revision;
      await discardCurrent("re-review");
      setOutcome(null);
      setRestoreOutcome(null);
      const specs = [
        ...(isPs4 && cfg.withArchive
          ? [{ archive_relative: cfg.archiveRel, member_path: cfg.memberPath, artifact_key: "primary" }]
          : []),
        ...(cfg.withDictionaryMember
          ? [{ archive_relative: cfg.dictionaryArchiveRel, member_path: cfg.dictionaryMemberPath, artifact_key: "dictionary" }]
          : []),
      ];
      const review =
        specs.length > 0
          ? await call("copied_target_review_archive_members", { projectId: forProject, root: cfg.root, specs })
          : await call("copied_target_review", { projectId: forProject, root: cfg.root });
      // Late arrival: the panel unmounted or the project changed while the review was in flight.
      // Nothing owns this handle any more — dispose of it now, never adopt it.
      if (gen !== genRef.current || !mountedRef.current || projectIdRef.current !== forProject) {
        await disposeHandle(review.handle, "arrived after the panel moved on");
        return null;
      }
      return { review, projectId: forProject, revision, config: cfg } satisfies BoundReview;
    },
    onSuccess: (b) => {
      if (!b) return;
      setBound(b);
      setMessage(null);
      void status.refetch();
    },
  });

  const doApply = useMutation({
    mutationFn: async (b: BoundReview) => {
      const out = await call("copied_target_apply", { projectId: b.projectId, handle: b.review.handle });
      return { out, forProject: b.projectId };
    },
    onSuccess: async ({ out, forProject }, b) => {
      await settleAppliedHandle(b.config.root);
      if (forProject === projectIdRef.current) setOutcome(out);
      await refresh(forProject);
    },
    onError: async (_e, b) => {
      // A refused or cancelled apply aborts the prepared set server-side; the handle is gone
      // either way. Nothing in the session changed, so the summary refresh does not gate the
      // error's display (a commit, above, waits for it: the revision moved).
      await settleAppliedHandle(b.config.root);
      void refresh(b.projectId);
    },
  });

  const doCancel = useMutation({
    mutationFn: (handle: number) => call("copied_target_cancel", { handle }),
    onSuccess: (did) => setMessage(did ? "Cancellation requested." : "Nothing to cancel for that handle."),
  });

  const doDiscard = useMutation({
    mutationFn: () => discardCurrent("discarded"),
    onSuccess: () => void status.refetch(),
  });

  const doDisposeListed = useMutation({
    mutationFn: (handle: number) => disposeHandle(handle, "listed by the root status"),
    onSuccess: () => void status.refetch(),
  });

  const doRecover = useMutation({
    mutationFn: () => call("copied_target_recover", { root: config.root }),
    onSuccess: async (r) => {
      setMessage(r.status.guidance);
      const mine = r.affected.find((b) => b.project_id === projectId);
      if (mine) setBaseline(mine);
      await refresh(projectId);
    },
  });

  const doRestore = useMutation({
    mutationFn: (txnId: string) => call("copied_target_restore", { root: config.root, txnId }),
    onSuccess: async (r) => {
      setRestoreOutcome(r);
      setOutcome(null);
      const mine = r.affected.find((b) => b.project_id === projectId);
      if (mine) setBaseline(mine);
      await refresh(projectId);
    },
  });

  const doReload = useMutation({
    mutationFn: (discardEdits: boolean) => call("copied_target_reload", { projectId, discardEdits }),
    onSuccess: async (b) => {
      setBaseline(b);
      await refresh(projectId);
    },
  });

  const busy =
    doReview.isPending || doApply.isPending || doDiscard.isPending || doRecover.isPending || doRestore.isPending || doReload.isPending;
  const err =
    doReview.error || doApply.error || doDiscard.error || doRecover.error || doRestore.error || doReload.error || createRoot.error || doCancel.error;

  const setCfg = (patch: Partial<CopiedTargetConfig>) => setConfig((c) => ({ ...c, ...patch }));

  return (
    <Card
      title="Save to a copied target"
      subtitle="Writes the edited files into a copied game tree through the coordinated transaction (review → apply → restore). Nothing here installs into a live game or claims in-game acceptance."
    >
      <div className="space-y-3">
        <RootRow
          root={config.root}
          setRoot={(r) => setCfg({ root: r })}
          frozen={frozen}
          lookupNote={rootLookup.data?.note ?? null}
          onCreate={(path) => createRoot.mutate(path)}
          creating={createRoot.isPending}
        />

        {isPs4 && (
          <div className="space-y-1 text-xs text-slate-400">
            <label className="flex items-center gap-2">
              <input
                type="checkbox"
                aria-label="Also replace the database member inside a copied archive"
                checked={config.withArchive}
                disabled={frozen}
                onChange={(e) => setCfg({ withArchive: e.target.checked })}
              />
              Also replace the database member inside a copied archive (same transaction)
            </label>
            {config.withArchive && (
              <div className="grid gap-1 sm:grid-cols-2">
                <input
                  aria-label="archive path relative to the root"
                  value={config.archiveRel}
                  disabled={frozen}
                  onChange={(e) => setCfg({ archiveRel: e.target.value })}
                  placeholder="archive path relative to the root"
                  className="rounded-md border border-slate-700 bg-slate-950 px-2 py-1 font-mono text-[11px] text-slate-100 focus:border-sky-600 focus:outline-none disabled:opacity-60"
                />
                <input
                  aria-label="member path inside the archive"
                  value={config.memberPath}
                  disabled={frozen}
                  onChange={(e) => setCfg({ memberPath: e.target.value })}
                  placeholder="member path inside the archive"
                  className="rounded-md border border-slate-700 bg-slate-950 px-2 py-1 font-mono text-[11px] text-slate-100 focus:border-sky-600 focus:outline-none disabled:opacity-60"
                />
                <p className="sm:col-span-2 text-slate-500">
                  Only an existing raw member is replaced; nothing is added, recompressed or installed. The
                  archive&apos;s per-row sort key is preserved but not understood, so the result stays unqualified.
                </p>
              </div>
            )}
          </div>
        )}
        <div className="space-y-1 text-xs text-slate-400">
          <label className="flex items-center gap-2">
            <input
              type="checkbox"
              aria-label="Also replace the dictionary member inside a copied archive"
              checked={config.withDictionaryMember}
              disabled={frozen}
              onChange={(e) => setCfg({ withDictionaryMember: e.target.checked })}
            />
            Also replace the localization dictionary member inside a copied archive (pending rename; same transaction)
          </label>
          {config.withDictionaryMember && (
            <div className="grid gap-1 sm:grid-cols-2">
              <input
                aria-label="dictionary archive path relative to the root"
                value={config.dictionaryArchiveRel}
                disabled={frozen}
                onChange={(e) => setCfg({ dictionaryArchiveRel: e.target.value })}
                placeholder="archive path relative to the root"
                className="rounded-md border border-slate-700 bg-slate-950 px-2 py-1 font-mono text-[11px] text-slate-100 focus:border-sky-600 focus:outline-none disabled:opacity-60"
              />
              <input
                aria-label="dictionary member path inside the archive"
                value={config.dictionaryMemberPath}
                disabled={frozen}
                onChange={(e) => setCfg({ dictionaryMemberPath: e.target.value })}
                placeholder="member path inside the archive"
                className="rounded-md border border-slate-700 bg-slate-950 px-2 py-1 font-mono text-[11px] text-slate-100 focus:border-sky-600 focus:outline-none disabled:opacity-60"
              />
              <p className="sm:col-span-2 text-slate-500">
                Needs a pending dictionary rename (Leagues → league → Rename via dictionary). The loose dictionary file
                is replaced as well; only an existing raw member is replaced.
              </p>
            </div>
          )}
        </div>
        {frozen && (
          <p className="text-xs text-slate-500">
            Destination settings are locked while a review is prepared — discard the review to change them.
          </p>
        )}

        {status.isError && <p className="text-sm text-rose-400">{String(status.error)}</p>}
        {status.data && (
          <StatusRow
            status={status.data}
            ownHandle={bound?.review.handle ?? null}
            onDispose={(h) => doDisposeListed.mutate(h)}
            disposing={doDisposeListed.isPending}
          />
        )}

        <div className="flex flex-wrap items-center gap-2">
          <Button onClick={() => doReview.mutate()} disabled={busy || config.root.trim() === "" || !p.dirty}>
            Review save
          </Button>
          {bound && (
            <>
              <Button onClick={() => doApply.mutate(bound)} disabled={busy || stale}>
                Apply to copied target
              </Button>
              <Button variant="secondary" onClick={() => doDiscard.mutate()} disabled={busy}>
                Discard review
              </Button>
            </>
          )}
          {doApply.isPending && bound && (
            <Button variant="secondary" onClick={() => doCancel.mutate(bound.review.handle)}>
              Cancel apply
            </Button>
          )}
          {status.data && needsRecovery(status.data) && (
            <Button variant="secondary" onClick={() => doRecover.mutate()} disabled={busy}>
              Run recovery
            </Button>
          )}
        </div>

        {!p.dirty && !bound && <p className="text-xs text-slate-500">No unsaved changes to package.</p>}

        {bound && <ReviewTable review={bound.review} staleReason={staleReason} />}

        {outcome && <ApplyOutcome outcome={outcome} />}
        {restoreOutcome && <RestoreOutcome outcome={restoreOutcome} />}

        {status.data && status.data.transactions.some((t) => t.restorable) && (
          <div className="space-y-1">
            <p className="text-xs font-semibold text-slate-300">Restore an earlier state (exact originals)</p>
            {status.data.transactions
              .filter((t) => t.restorable)
              .map((t) => (
                <div key={t.txn_id} className="flex items-center gap-2 text-xs text-slate-400">
                  <span className="font-mono">{t.txn_id}</span>
                  <span>{t.state}</span>
                  <span>
                    {t.destinations.length} file{t.destinations.length === 1 ? "" : "s"}
                  </span>
                  <Button variant="secondary" onClick={() => doRestore.mutate(t.txn_id)} disabled={busy || !!bound}>
                    Restore
                  </Button>
                </div>
              ))}
            {bound && <p className="text-xs text-slate-500">Discard the prepared review before restoring.</p>}
          </div>
        )}

        {baseline?.stale_baseline && (
          <div className="rounded border border-amber-800 bg-amber-950/30 p-2 text-xs text-amber-200">
            <p>
              The files on disk changed underneath this session ({baseline.stale_baseline}). Your edits are kept;
              reload to adopt the files (discarding the edits) or keep working without a copied-target save.
            </p>
            <div className="mt-2 flex gap-2">
              <Button variant="secondary" onClick={() => doReload.mutate(false)} disabled={busy}>
                Reload (only if clean)
              </Button>
              <Button variant="secondary" onClick={() => doReload.mutate(true)} disabled={busy}>
                Reload and discard edits
              </Button>
            </div>
          </div>
        )}

        {cleanupFailures.length > 0 && (
          <ul className="space-y-1 text-xs text-amber-300" aria-label="cleanup failures">
            {cleanupFailures.map((f, i) => (
              <li key={i}>{f}</li>
            ))}
          </ul>
        )}
        {message && <p className="text-xs text-slate-300">{message}</p>}
        {err && <p className="text-sm text-rose-400">{String(err)}</p>}
      </div>
    </Card>
  );
}

function RootRow({
  root,
  setRoot,
  frozen,
  lookupNote,
  onCreate,
  creating,
}: {
  root: string;
  setRoot: (r: string) => void;
  frozen: boolean;
  lookupNote: string | null;
  onCreate: (path: string) => void;
  creating: boolean;
}) {
  const [newRoot, setNewRoot] = useState("");
  return (
    <div className="space-y-1">
      <label className="block text-xs text-slate-400" htmlFor="copied-root">
        Copied-target root (a directory created and marked by Studio; the open files must lie inside it)
      </label>
      <input
        id="copied-root"
        value={root}
        disabled={frozen}
        onChange={(e) => setRoot(e.target.value)}
        placeholder="copied root path"
        className="w-full rounded-md border border-slate-700 bg-slate-950 px-2 py-1.5 font-mono text-xs text-slate-100 placeholder-slate-600 focus:border-sky-600 focus:outline-none disabled:opacity-60"
      />
      {lookupNote && root === "" && <p className="text-xs text-amber-300/80">{lookupNote}</p>}
      <details className="text-xs text-slate-500">
        <summary className="cursor-pointer">Create a new copied root…</summary>
        <div className="mt-1 flex items-center gap-2">
          <input
            aria-label="new copied root path"
            value={newRoot}
            onChange={(e) => setNewRoot(e.target.value)}
            placeholder="new, empty directory path"
            className="flex-1 rounded-md border border-slate-700 bg-slate-950 px-2 py-1 font-mono text-xs text-slate-100 placeholder-slate-600 focus:border-sky-600 focus:outline-none"
          />
          <Button variant="secondary" onClick={() => onCreate(newRoot)} disabled={creating || frozen || newRoot.trim() === ""}>
            Create
          </Button>
        </div>
        <p className="mt-1">Only a directory created this way accepts saves. An installed game folder is never marked in place.</p>
      </details>
    </div>
  );
}

/// The handle (prepared or applying) that owns the root's active transaction, if the active
/// transaction is a live handle rather than an interrupted one left behind by an earlier run.
function activeHandle(status: CopiedTargetStatusDto) {
  return status.active_txn_id == null
    ? null
    : (status.pending_handles.find((h) => h.txn_id === status.active_txn_id) ?? null);
}

/// Recovery is for a transaction that blocks the root with nobody holding it. A root whose
/// active transaction is a prepared review or a running apply is not interrupted: offering
/// recovery there would abort the review the panel (or another panel) still owns.
function needsRecovery(status: CopiedTargetStatusDto): boolean {
  return !status.apply_allowed && status.active_txn_id != null && activeHandle(status) == null;
}

function StatusRow({
  status,
  ownHandle,
  onDispose,
  disposing,
}: {
  status: CopiedTargetStatusDto;
  ownHandle: number | null;
  onDispose: (handle: number) => void;
  disposing: boolean;
}) {
  const others = status.pending_handles.filter((h) => h.handle !== ownHandle);
  // The engine reports a prepared transaction as "blocking" the root (no second review may
  // start), which is the normal state while this panel holds a review — say so, rather than
  // presenting the panel's own review as a blocked root in need of recovery (LEAGUE-SMOKE-1).
  const active = activeHandle(status);
  const pill: { tone: "ok" | "warn" | "error"; text: string } = active
    ? active.in_progress
      ? { tone: "warn", text: "applying" }
      : active.handle === ownHandle
        ? { tone: "ok", text: "review prepared" }
        : { tone: "warn", text: "review pending" }
    : status.blocked
      ? { tone: "error", text: "blocked" }
      : status.apply_allowed
        ? { tone: "ok", text: "ready" }
        : { tone: "warn", text: "busy" };
  return (
    <div className="space-y-1 text-xs text-slate-400">
      <p>
        <Pill tone={pill.tone}>{pill.text}</Pill>{" "}
        {status.guidance}
        {status.staging_leftovers.length > 0 && (
          <span className="text-amber-300"> · {status.staging_leftovers.length} staging leftover(s) reported</span>
        )}
      </p>
      {others.length > 0 && (
        <ul aria-label="other prepared reviews" className="space-y-0.5">
          {others.map((h) => (
            <li key={h.handle} className="flex items-center gap-2">
              <span>
                Prepared review handle {h.handle} for project {h.project_id} ({h.in_progress ? "applying" : "not owned by this panel"})
              </span>
              {!h.in_progress && (
                <Button variant="secondary" onClick={() => onDispose(h.handle)} disabled={disposing}>
                  Discard
                </Button>
              )}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

function ReviewTable({ review, staleReason }: { review: CopiedTargetReviewDto; staleReason: string | null }) {
  return (
    <div className="space-y-2 rounded border border-slate-800 bg-slate-950/40 p-2" aria-label="prepared review">
      <p className="text-xs text-slate-400">
        Review handle {review.handle} · transaction <span className="font-mono">{review.txn_id}</span> · mode {review.mode} ·{" "}
        <Pill tone="warn">{review.qualification_state}</Pill>
        {!review.live_install_allowed && <span> · live install not allowed</span>}
      </p>
      {staleReason && (
        <p className="text-xs text-amber-300" role="status">
          This review is stale: {staleReason}. Apply is disabled — discard and review again.
        </p>
      )}
      <table className="w-full text-left text-xs">
        <thead className="uppercase tracking-wide text-slate-500">
          <tr>
            <th className="py-1 pr-2">Artifact</th>
            <th className="py-1 pr-2">Destination</th>
            <th className="py-1 pr-2">Prior</th>
            <th className="py-1 pr-2">Output</th>
            <th className="py-1 pr-2">Qualification</th>
          </tr>
        </thead>
        <tbody>
          {review.artifacts.map((a) => (
            <tr key={a.key} className="border-t border-slate-800/60 align-top">
              <td className="py-1 pr-2 text-slate-200">
                {a.label}
                {a.dependencies.length > 0 && <span className="text-slate-500"> (depends on {a.dependencies.join(", ")})</span>}
              </td>
              <td className="py-1 pr-2 font-mono text-slate-400">{a.destination}</td>
              <td className="py-1 pr-2 font-mono text-slate-500">{a.prior ? `${a.prior.len} B ${a.prior.sha256.slice(0, 12)}…` : "absent"}</td>
              <td className="py-1 pr-2 font-mono text-slate-300">
                {a.output.len} B {a.output.sha256.slice(0, 12)}…
              </td>
              <td className="py-1 pr-2 text-slate-400">
                {a.qualification_state}
                {a.qualification_items.map((q) => (
                  <div key={q.subject} className="text-slate-500">
                    {q.subject}: {q.status} — {q.note}
                  </div>
                ))}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {review.bound_inputs.length > 0 && (
        <p className="text-xs text-slate-500">Bound inputs: {review.bound_inputs.map((b) => `${b.kind} ${b.path}`).join("; ")}</p>
      )}
    </div>
  );
}

function ApplyOutcome({ outcome }: { outcome: CopiedTargetApplyDto }) {
  const ok = outcome.state === "committed";
  return (
    <div className={"rounded border p-2 text-xs " + (ok ? "border-emerald-800 bg-emerald-950/30" : "border-rose-800 bg-rose-950/30")}>
      <p className={ok ? "text-emerald-300" : "text-rose-300"}>
        Transaction <span className="font-mono">{outcome.txn_id}</span>: {outcome.state}
        {outcome.applied_unqualified && " — written to the copied tree as unqualified artifacts (not an in-game installation)"}
      </p>
      <ul className="mt-1 text-slate-400">
        {outcome.destinations.map((d) => (
          <li key={d.key} className="font-mono">
            {d.destination}
            {d.identity ? ` → ${d.identity.len} B ${d.identity.sha256.slice(0, 12)}…` : ""}
          </li>
        ))}
      </ul>
      <p className="mt-1 text-slate-500">
        Session baseline: primary {outcome.project.primary}
        {outcome.project.roster ? `, roster ${outcome.project.roster}` : ""}
        {outcome.project.dirty ? " (still dirty)" : " (clean)"}
        {outcome.project.refresh_error ? ` — ${outcome.project.refresh_error}` : ""}
      </p>
      <p className="text-slate-500">Recovery material: {outcome.recovery_dir}</p>
    </div>
  );
}

function RestoreOutcome({ outcome }: { outcome: CopiedTargetRestoreDto }) {
  return (
    <div className="rounded border border-sky-800 bg-sky-950/30 p-2 text-xs">
      <p className="text-sky-200">
        Restore of <span className="font-mono">{outcome.txn_id}</span>: {outcome.state}
      </p>
      <ul className="mt-1 text-slate-400">
        {outcome.destinations.map((d) => (
          <li key={d.key} className="font-mono">
            {d.destination}
            {d.identity ? ` → ${d.identity.len} B ${d.identity.sha256.slice(0, 12)}…` : " (absent)"}
          </li>
        ))}
      </ul>
      {outcome.affected.map((b) => (
        <p key={b.project_id} className="text-slate-500">
          Project {b.project_id}: {b.dirty ? "edits kept" : "reloaded"}
          {b.stale_baseline ? ` — ${b.stale_baseline}` : ""}
        </p>
      ))}
    </div>
  );
}

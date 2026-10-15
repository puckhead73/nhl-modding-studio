import { useEffect, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import type { IsoApplyDto } from "../bindings/IsoApplyDto";
import type { IsoPlanDto } from "../bindings/IsoPlanDto";
import type { IsoRestoreDto } from "../bindings/IsoRestoreDto";
import { Button, Card, Pill } from "./primitives";

/// Write the staged edits back into the Xbox 360 disc image the project was opened from, and
/// restore them. Only shown for a project opened through the image route.
///
/// Ownership (R2 F2): every call names the project, never a path. The plan shown is a
/// server-held *review* (`review_id`) computed at one workspace revision; "Yes, write" applies
/// exactly that review, and the server refuses it when the project is dirty or was edited
/// since, when the staged bytes changed after the review, when the platform's emulator is
/// running, or when another operation holds the image. The confirm step re-checks the review
/// against the project's current revision and dirty flag before offering the button, and the
/// panel drops late results that belong to another project or an older review.
export function DiscImagePanel() {
  const { project, iso } = useProject();
  const qc = useQueryClient();
  const projectId = project?.project_id ?? -1;
  const root = iso?.root ?? null;
  const [apply, setApply] = useState<IsoApplyDto | null>(null);
  const [restore, setRestore] = useState<IsoRestoreDto | null>(null);
  /// The review the user confirmed (bound at "Write…" time; the confirm compares it to the
  /// current plan and project).
  const [confirm, setConfirm] = useState<{ reviewId: bigint; revision: string } | null>(null);
  const genRef = useRef(0);
  const mountedRef = useRef(true);
  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      genRef.current += 1;
    };
  }, []);
  useEffect(() => {
    genRef.current += 1;
    setApply(null);
    setRestore(null);
    setConfirm(null);
  }, [projectId, root]);

  const plan = useQuery({
    queryKey: ["iso-plan", projectId, root, project?.workspace_revision, project?.dirty],
    queryFn: () => call("iso_plan", { projectId }),
    enabled: root != null && projectId >= 0,
  });
  const records = useQuery({
    queryKey: ["iso-records", projectId, root],
    queryFn: () => call("iso_records", { projectId }),
    enabled: root != null && projectId >= 0,
  });
  const status = useQuery({
    queryKey: ["iso-status", projectId, root],
    queryFn: () => call("iso_status", { projectId }),
    enabled: root != null && projectId >= 0,
  });

  const doApply = useMutation({
    mutationFn: async (reviewId: bigint) => {
      const gen = genRef.current;
      const r = await call("iso_apply", { projectId, reviewId });
      return { r, gen };
    },
    onSettled: () => {
      void qc.invalidateQueries({ queryKey: ["iso-plan"] });
      void qc.invalidateQueries({ queryKey: ["iso-records"] });
      void qc.invalidateQueries({ queryKey: ["iso-status"] });
    },
    onSuccess: ({ r, gen }) => {
      if (mountedRef.current && gen === genRef.current) {
        setApply(r);
        setRestore(null);
        setConfirm(null);
      }
    },
  });
  const doRestore = useMutation({
    mutationFn: async (recordsDir: string) => {
      const gen = genRef.current;
      const r = await call("iso_restore", { projectId, recordsDir });
      return { r, gen };
    },
    onSettled: () => {
      void qc.invalidateQueries({ queryKey: ["iso-plan"] });
      void qc.invalidateQueries({ queryKey: ["iso-records"] });
      void qc.invalidateQueries({ queryKey: ["iso-status"] });
    },
    onSuccess: ({ r, gen }) => {
      if (mountedRef.current && gen === genRef.current) {
        setRestore(r);
        setApply(null);
      }
    },
  });

  if (!iso || !project) return null;
  const dirty = project.dirty;
  const p: IsoPlanDto | undefined = plan.data;
  const busy = doApply.isPending || doRestore.isPending;
  const inFlight = status.data?.in_flight ?? null;
  // The review is current only when it was computed for this project at its current revision
  // with nothing unsaved; otherwise the write must be re-reviewed.
  const reviewCurrent = !!p && p.project_id === project.project_id && p.workspace_revision === project.workspace_revision && !p.dirty && !dirty;
  const confirmCurrent = !!confirm && reviewCurrent && !!p && confirm.reviewId === p.review_id && confirm.revision === project.workspace_revision;
  const applyRecordsDirs = Array.from(new Set((records.data ?? []).map((r) => r.records_dir)));

  return (
    <Card
      title="Disc image"
      subtitle={`Write the saved staging files back into ${iso.image} — same-length ranges only, hash-bound, with a verified backup per range.`}
    >
      <div className="mb-2 flex flex-wrap items-center gap-2 text-xs text-slate-400">
        <Pill tone="warn">writes the image in place</Pill>
        <span>
          staging <span className="font-mono">{iso.root}</span> · {iso.layout} · staged {iso.staged_at}
        </span>
        {inFlight && <Pill tone="warn">busy: {inFlight}</Pill>}
      </div>
      {dirty && (
        <p className="mb-2 text-xs text-amber-300/90">
          This project has unsaved edits. Save first: the image receives the staging files as they are on disk.
        </p>
      )}
      {plan.isError && <p className="mb-2 text-sm text-rose-400">{String(plan.error)}</p>}
      {p && (
        <table className="w-full text-left text-xs" data-testid="iso-plan" data-review={String(p.review_id)}>
          <thead className="text-slate-500">
            <tr>
              <th className="py-1 pr-2">file</th>
              <th className="py-1 pr-2">state</th>
              <th className="py-1 pr-2">image range</th>
              <th className="py-1 pr-2">prior → replacement</th>
            </tr>
          </thead>
          <tbody>
            {p.files.map((f) => (
              <tr key={f.role} className="border-t border-slate-800">
                <td className="py-1 pr-2 font-mono">{f.staged}</td>
                <td className="py-1 pr-2">
                  {f.changed ? <Pill tone="info">changed</Pill> : <Pill tone="neutral">unchanged</Pill>}
                </td>
                <td className="py-1 pr-2 font-mono text-slate-400">
                  {f.changed ? `${f.member_path}${f.entry_path && f.entry_path !== f.member_path ? " › " + f.entry_path : ""} [${f.start}, ${f.end})` : "—"}
                </td>
                <td className="py-1 pr-2 font-mono text-slate-400">
                  {f.changed ? `${f.prior_sha256?.slice(0, 10)}… → ${f.replacement_sha256?.slice(0, 10)}…` : f.staged_sha256.slice(0, 10) + "…"}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
      {p && (
        <p className="mt-1 text-[11px] text-slate-500">
          review {String(p.review_id)} at revision <span className="font-mono">{p.workspace_revision.slice(0, 16)}</span>
          {!reviewCurrent && !dirty && p.project_id === project.project_id && p.workspace_revision !== project.workspace_revision && (
            <> · <span className="text-amber-300">the project changed since this review; it is being re-reviewed</span></>
          )}
        </p>
      )}
      <div className="mt-3 flex flex-wrap items-center gap-2">
        {!confirmCurrent ? (
          <Button
            onClick={() => p && setConfirm({ reviewId: p.review_id, revision: project.workspace_revision })}
            disabled={busy || !reviewCurrent || !p || p.writes === 0 || !!inFlight}
          >
            Write to disc image…
          </Button>
        ) : (
          <>
            <span className="text-xs text-amber-300">
              This modifies {iso.image} in place ({p?.writes ?? 0} range{p?.writes === 1 ? "" : "s"}, review {String(confirm!.reviewId)}). A
              backup of each range is recorded first. Continue?
            </span>
            <Button onClick={() => doApply.mutate(confirm!.reviewId)} disabled={busy || !confirmCurrent}>
              {doApply.isPending ? "Writing…" : "Yes, write"}
            </Button>
            <Button variant="secondary" onClick={() => setConfirm(null)} disabled={busy}>
              Cancel
            </Button>
          </>
        )}
        {p && p.writes === 0 && !dirty && (
          <span className="text-[11px] text-slate-500">Nothing to write: the staging files match the image.</span>
        )}
      </div>
      {doApply.isError && <p className="mt-2 text-sm text-rose-400">{String(doApply.error)}</p>}
      {apply && apply.outcome === "committed" && (
        <div className="mt-3 rounded-md border border-emerald-700/40 bg-emerald-900/20 p-2 text-xs text-emerald-200" data-testid="iso-applied">
          Written and verified: {apply.records.map((r) => r.entry_path).join(", ")}
          {apply.unchanged.length > 0 && <> (unchanged: {apply.unchanged.join(", ")})</>} · records{" "}
          <span className="font-mono">{apply.records_dir}</span>
        </div>
      )}
      {apply && apply.outcome !== "committed" && (
        <div
          className={`mt-3 rounded-md border p-2 text-xs ${apply.outcome === "split" ? "border-rose-700/60 bg-rose-900/20 text-rose-100" : "border-amber-700/60 bg-amber-900/20 text-amber-100"}`}
          data-testid="iso-apply-failed"
        >
          <div className="font-semibold">{apply.outcome === "split" ? "The write failed and the image is split" : "The write failed; the image is as it was"}</div>
          <div>{apply.message}</div>
          <ul className="mt-1 list-disc pl-5">
            {apply.attempted.map((a) => (
              <li key={a.entry_path}>
                <span className="font-mono">{a.entry_path}</span>: image holds <Pill tone={a.state === "prior" ? "ok" : "warn"}>{a.state}</Pill>
                {a.restore_error && <> — {a.restore_error}</>} · record <span className="font-mono">{a.record.record_dir}</span>
              </li>
            ))}
          </ul>
        </div>
      )}
      {doRestore.isError && <p className="mt-2 text-sm text-rose-400">{String(doRestore.error)}</p>}
      {restore && (
        <div className="mt-3 rounded-md border border-slate-700 bg-slate-950 p-2 text-xs text-slate-200" data-testid="iso-restored">
          {restore.items.map((i) => (
            <div key={i.entry_path}>
              <span className="font-mono">{i.entry_path}</span>: {i.outcome}
            </div>
          ))}
          {restore.all_prior ? "The image holds its original bytes at every recorded range." : "Some ranges were refused; nothing unknown was overwritten."}
        </div>
      )}
      {applyRecordsDirs.length > 0 && (
        <div className="mt-3">
          <div className="mb-1 text-xs font-semibold text-slate-300">Recorded writes</div>
          <table className="w-full text-left text-xs">
            <thead className="text-slate-500">
              <tr>
                <th className="py-1 pr-2">records</th>
                <th className="py-1 pr-2">file</th>
                <th className="py-1 pr-2">recorded</th>
                <th className="py-1 pr-2">image now holds</th>
                <th className="py-1 pr-2"></th>
              </tr>
            </thead>
            <tbody>
              {(records.data ?? []).map((r) => (
                <tr key={r.record_dir} className="border-t border-slate-800">
                  <td className="py-1 pr-2 font-mono text-slate-400">{r.records_dir.split(/[\\/]/).slice(-1)[0]}</td>
                  <td className="py-1 pr-2 font-mono">{r.entry_path}</td>
                  <td className="py-1 pr-2">{r.status}</td>
                  <td className="py-1 pr-2">
                    <Pill tone={r.on_image === "replacement" ? "info" : r.on_image === "prior" ? "ok" : "warn"}>{r.on_image}</Pill>
                    {r.on_image === "mixed" && (
                      <span className="ml-1 text-[11px] text-slate-500" title="The range holds a mixture of the old and new bytes but this record's write completed: a later external change, not an interrupted write. Studio will not overwrite it; inspect the range by hand.">
                        changed after this write — not restorable here
                      </span>
                    )}
                  </td>
                  <td className="py-1 pr-2">
                    {/* Restore is offered only for states the record's lifecycle authorizes: the
                        verified replacement, or an interrupted apply/restore (`partial`) on an
                        unfinished record. `mixed` / `unknown` are external changes: never offered. */}
                    {(r.on_image === "replacement" || r.on_image === "partial") && (
                      <Button variant="secondary" onClick={() => doRestore.mutate(r.records_dir)} disabled={busy || !!inFlight}>
                        Restore
                      </Button>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      {status.data && (
        <p className="mt-2 text-[11px] text-slate-500">{status.data.note}</p>
      )}
    </Card>
  );
}

import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "react-router-dom";
import { call } from "../lib/client";
import { useIsSaveEditor, useProject } from "../lib/project";
import { EntityList, type Column } from "../components/EntityList";

/// The Leagues master list. Rows come from the server's per-target league catalog: on PS4 titles
/// each row is a `stockleagues` record (its name is a localization key, shown as a key until a
/// dictionary resolves it); on older titles each row is a league code carried by the teams,
/// labelled only where a measured vocabulary applies. Rows are keyed by the native code — never by
/// the text, which may repeat (NHL 21 has three `CUSTOM_TEAMS` rows).
type Row = {
  code: number;
  display: string;
  nameKey: string | null;
  confidence: string;
  members: number;
};

export function LeaguesPage() {
  const { project } = useProject();
  const projectId = project!.project_id;

  const catalog = useQuery({
    queryKey: ["leagues", projectId],
    queryFn: () => call("list_leagues", { projectId }),
  });

  const rows: Row[] = useMemo(
    () =>
      (catalog.data?.leagues ?? []).map((l) => ({
        code: l.code,
        display: l.display,
        nameKey: l.name_key,
        confidence: l.confidence ?? "raw",
        members: l.member_teams,
      })),
    [catalog.data],
  );

  const columns: Column<Row>[] = [
    {
      key: "code",
      label: "Code",
      width: "44px",
      sort: (r) => r.code,
      render: (r) => r.code,
      cellClass: () => "tabular-nums text-slate-500",
    },
    {
      key: "display",
      label: "League",
      width: "1fr",
      sort: (r) => r.display,
      render: (r) => r.display,
      // A bare localization key or a raw code reads as code, a resolved label as a name.
      cellClass: (r) =>
        r.confidence === "native" || r.confidence === "raw"
          ? "font-mono text-xs text-amber-200/90"
          : "font-medium",
    },
    {
      key: "members",
      label: "Teams",
      width: "48px",
      sort: (r) => r.members,
      render: (r) => r.members,
      cellClass: () => "tabular-nums text-slate-400",
    },
  ];

  const saveEditor = useIsSaveEditor();
  const note = catalog.data?.unresolved_reason;
  const dict = catalog.data?.dictionary ?? null;
  const toolbar = (
    <span className="flex items-center gap-2 text-xs">
      {/* Imagery and commentary read the game installation; a Save Editor session has none. */}
      {!saveEditor && <Link to="imagery" className="text-sky-300 hover:underline" title="Playoff-tree logos, trophies and the PS4 registry types keyed by league — preview and replace">
        presentation imagery
      </Link>}
      {!saveEditor && <Link to="commentary" className="text-sky-300 hover:underline" title="Read-only: the title's speech events, the sentences sharing an event's name and their candidate samples (Xbox 360 / PS3 NHL 11–Legacy); the context listing also covers PS4 NHL 15–21">
        commentary
      </Link>}
      {note && (
        <span className="text-amber-300/80" title={note}>
          codes shown raw
        </span>
      )}
      {dict ? (
        <>
          <span className="text-slate-500" title={`${dict.path} · ${dict.sha256.slice(0, 12)}…`}>
            names via {dict.locale} dictionary ({dict.rows.toLocaleString()} rows)
          </span>
          <DictionaryDetach projectId={projectId} />
        </>
      ) : (
        <DictionaryAttach projectId={projectId} />
      )}
    </span>
  );

  return (
    <EntityList
      title="Leagues"
      rows={rows}
      getId={(r) => r.code}
      columns={columns}
      searchOf={(r) => `${r.display} ${r.nameKey ?? ""} ${r.code}`}
      searchPlaceholder="Search league / key / code…"
      isLoading={catalog.isLoading}
      error={catalog.error}
      initialSort="code"
      toolbar={toolbar}
    />
  );
}

/// Deliberately drop the attached dictionary and any pending rename planned on it. This is the
/// only way a dictionary leaves a project: a reload that cannot re-read the file keeps it attached
/// and reports the problem instead (R1 F2).
export function DictionaryDetach({ projectId }: { projectId: number }) {
  const qc = useQueryClient();
  const { update } = useProject();
  const detach = useMutation({
    mutationFn: async () => {
      const forProject = projectId;
      await call("detach_dictionary", { projectId: forProject });
      return forProject;
    },
    onSuccess: async (forProject) => {
      void qc.invalidateQueries();
      const summary = await call("project_summary", { projectId: forProject });
      // The shared boundary drops a summary for a project that is no longer the open one.
      update(summary);
    },
  });
  return (
    <span className="flex items-center gap-1">
      <button
        className="rounded border border-slate-700 px-1.5 py-0.5 text-[11px] text-slate-300 hover:border-sky-600 disabled:opacity-50"
        onClick={() => detach.mutate()}
        disabled={detach.isPending}
        title="Detach the dictionary from this project. A pending rename planned on it is dropped; the file on disk is not touched."
      >
        detach
      </button>
      {detach.isError && <span className="text-rose-400">{String(detach.error)}</span>}
    </span>
  );
}

/// No dictionary is attached: names show as keys (PS4) or vocabulary labels (older titles). Let
/// the user point at the target's own localization file (the install's `eng_us.db` /
/// `nhl_eng_us.db`); anything that is not a verified dictionary is refused server-side.
export function DictionaryAttach({ projectId }: { projectId: number }) {
  const qc = useQueryClient();
  const [path, setPath] = useState("");
  const attach = useMutation({
    mutationFn: (p: string) => call("attach_dictionary", { projectId, path: p }),
    onSuccess: () => qc.invalidateQueries(),
  });
  return (
    <span className="flex items-center gap-1">
      <span className="text-amber-300/80" title="On-screen league names live in this title's dictionary; attach it to see the game's text and to rename via the dictionary.">
        no dictionary
      </span>
      <input
        value={path}
        onChange={(e) => setPath(e.target.value)}
        placeholder="dictionary path (eng_us.db / nhl_eng_us.db)"
        className="w-40 rounded border border-slate-700 bg-slate-950 px-1 py-0.5 font-mono text-[11px] text-slate-100 placeholder-slate-600 focus:border-sky-600 focus:outline-none"
      />
      <button
        className="rounded border border-slate-700 px-1.5 py-0.5 text-[11px] text-slate-300 hover:border-sky-600 disabled:opacity-50"
        onClick={() => attach.mutate(path)}
        disabled={attach.isPending || path.trim() === ""}
      >
        attach
      </button>
      {attach.isError && <span className="text-rose-400">{String(attach.error)}</span>}
    </span>
  );
}

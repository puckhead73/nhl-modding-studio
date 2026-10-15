// About & licences (receipt 46): Studio's own licence, the candidate's release status and the third-party
// notices shipped beside the executable. Everything shown is read from the archive's own files, offline; a
// missing file is named by its path, never papered over. Attribution only — no product marketing here.
import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { Button, Card, ErrorBanner, Pill, Spinner } from "../components/primitives";
import exisoLicence from "../../../crates/studio-domain/vendor/extract-xiso/LICENSE.TXT?raw";

function XboxExtractorNotice() {
  return <Card title="Xbox disc extraction — extract-xiso 2.7.1" className="mt-4">
    <p className="text-sm text-slate-300">This product includes software developed by in &lt;in@fishtank.com&gt;.</p>
    <details className="mt-2 text-sm text-slate-300"><summary>Extractor licence</summary><pre className="mt-2 max-h-80 overflow-auto whitespace-pre-wrap text-xs">{exisoLicence}</pre></details>
  </Card>;
}

const preCls =
  "max-h-[28rem] overflow-auto whitespace-pre-wrap break-words rounded-md border border-slate-800 bg-slate-950 p-3 font-mono text-[11px] leading-relaxed text-slate-200 focus:border-sky-600 focus:outline-none";

function LicenceText({ path }: { path: string }) {
  const q = useQuery({ queryKey: ["about_licence_file", path], queryFn: () => call("about_licence_file", { path }) });
  if (q.isPending) return <Spinner label="Reading…" />;
  if (q.isError) return <ErrorBanner title={`Couldn't read ${path}`} message={String(q.error)} />;
  return (
    <pre className={preCls} tabIndex={0} aria-label={`Text of ${path}`}>
      {q.data}
    </pre>
  );
}

export function AboutContent() {
  const q = useQuery({ queryKey: ["about_studio"], queryFn: () => call("about_studio", {}) });
  const [open, setOpen] = useState<string | null>(null);
  if (q.isPending) return <Spinner label="Reading the packaged notices…" />;
  if (q.isError) return <ErrorBanner title="Couldn't read the About information" message={String(q.error)} />;
  const a = q.data;
  return (
    <div className="flex flex-col gap-4">
      <Card title="NHL Modding Studio" subtitle={`Version ${a.version}${a.candidate ? ` · ${a.candidate}` : ""}`}>
        <p className="text-xs text-slate-300">
          A fan-made tool for editing copies of EA NHL games you own. It ships no EA game data of its own; the
          games, their names and their artwork belong to their owners.
        </p>
        {a.packaged && (
          <div className="mt-3" role="status">
            {a.release_ready ? (
              <Pill tone="ok">Release candidate: every release check in its build record passed</Pill>
            ) : (
              <>
                <Pill tone="warn">Internal review build — not a release</Pill>
                {a.release_blockers.length > 0 && (
                  <ul className="mt-2 list-disc pl-5 text-xs text-amber-200/90">
                    {a.release_blockers.map((b) => (
                      <li key={b}>{b}</li>
                    ))}
                  </ul>
                )}
              </>
            )}
          </div>
        )}
      </Card>

      {a.problems.length > 0 && (
        <ErrorBanner
          title={a.packaged ? "Files this build should include are missing" : "Licence files are not available in this build"}
          message={
            <ul className="list-disc pl-4">
              {a.problems.map((p) => (
                <li key={p}>{p}</li>
              ))}
            </ul>
          }
        />
      )}

      <Card title="Studio's licence" subtitle={`Studio-owned source code: ${a.studio_licence_spdx} License. Third-party components keep their own licences (below).`}>
        {a.studio_licence_text ? (
          <pre className={preCls} tabIndex={0} aria-label="Text of Studio's licence">
            {a.studio_licence_text}
          </pre>
        ) : (
          <p className="text-xs text-slate-400">
            This build does not include Studio's licence file.
            {a.packaged && !a.release_ready ? " Its copyright line has not been recorded yet, so the file is withheld rather than filled with a placeholder." : ""}
          </p>
        )}
      </Card>

      <Card title="Third-party notices" subtitle="Software included in this build and where each licence text is.">
        {a.notices ? (
          <pre className={preCls} tabIndex={0} aria-label="Third-party notices">
            {a.notices}
          </pre>
        ) : (
          <p className="text-xs text-slate-400">The notices file is not available in this build.</p>
        )}
      </Card>

      {a.licence_files.length > 0 && (
        <Card title="Licence texts" subtitle={`${a.licence_files.length} files shipped in this build. Choose one to read it.`}>
          <div className="flex flex-col gap-3 md:flex-row">
            <ul className="max-h-[28rem] shrink-0 overflow-auto text-xs md:w-72" aria-label="Licence files">
              {a.licence_files.map((f) => (
                <li key={f.path}>
                  <button
                    type="button"
                    onClick={() => setOpen(f.path)}
                    aria-pressed={open === f.path}
                    className={`w-full truncate rounded px-2 py-1 text-left hover:bg-slate-800 focus:outline-none focus:ring-1 focus:ring-sky-600 ${open === f.path ? "bg-slate-800 text-sky-200" : "text-slate-300"}`}
                    title={f.path}
                  >
                    {f.path}
                  </button>
                </li>
              ))}
            </ul>
            <div className="min-w-0 flex-1">{open ? <LicenceText path={open} /> : <p className="text-xs text-slate-500">No file selected.</p>}</div>
          </div>
        </Card>
      )}

      <p className="text-[11px] text-slate-500">Read from {a.root || "(unresolved folder)"}.</p>
    </div>
  );
}

/** Standalone route (reachable before a project is open). */
export function AboutPage() {
  const navigate = useNavigate();
  return (
    <main className="h-screen overflow-y-auto bg-slate-950 px-6 py-6">
      <div className="mx-auto max-w-4xl">
        <header className="mb-4 flex items-center justify-between">
          <h1 className="text-xl font-semibold text-slate-100">About &amp; licences</h1>
          <Button variant="secondary" onClick={() => navigate(-1)}>
            Back
          </Button>
        </header>
        <AboutContent />
        <XboxExtractorNotice />
      </div>
    </main>
  );
}

/** Settings subtab. */
export function SettingsAboutPage() {
  return (
    <div className="mx-auto max-w-4xl">
      <header className="mb-4">
        <h1 className="text-xl font-semibold text-slate-100">About &amp; licences</h1>
      </header>
      <AboutContent />
      <XboxExtractorNotice />
    </div>
  );
}

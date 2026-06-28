import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { useMutation } from "@tanstack/react-query";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { call, type Platform, type Role } from "../lib/client";
import { useProject } from "../lib/project";
import { Button, ErrorBanner, Pill } from "../components/primitives";
import headerLogo from "../assets/ui/NHL Modding Studio Header.png";
import ps3Img from "../assets/ui/PS3.png";
import xboxImg from "../assets/ui/360.png";
import recompImg from "../assets/ui/legacyrecomp.png";

type Step = "console" | "file";

const CONSOLES: { id: Platform; label: string; image: string; recommended?: boolean; disabled?: boolean }[] = [
  { id: "recomp", label: "NHL Legacy Recompiled", image: recompImg, recommended: true },
  { id: "ps3", label: "PS3", image: ps3Img },
  { id: "xbox", label: "Xbox 360", image: xboxImg },
];

export function OpenProject() {
  const { open } = useProject();
  const navigate = useNavigate();

  const [step, setStep] = useState<Step>("console");
  const [platform, setPlatform] = useState<Platform>("recomp");
  const [role, setRole] = useState<Role>("nhlng");
  const [dbPath, setDbPath] = useState("");
  const [advanced, setAdvanced] = useState(false);
  const [metaXml, setMetaXml] = useState("");
  const [defsRoot, setDefsRoot] = useState("");

  const load = useMutation({
    mutationFn: () =>
      advanced
        ? call("load_project", { dbPath, platform, role, metaXml, defsRoot })
        : call("open_project", { dbPath, platform, role }),
    onSuccess: (summary) => {
      open(summary, dbPath);
      navigate("/app/dashboard");
    },
  });

  async function browse() {
    const picked = await openDialog({
      multiple: false,
      directory: false,
      title: "Pick the nhlng.db (or a save container)",
    });
    if (typeof picked === "string") setDbPath(picked);
  }

  const input =
    "w-full rounded-md border border-slate-700 bg-slate-950 px-2 py-1.5 text-sm text-slate-100 placeholder-slate-600 focus:border-sky-600 focus:outline-none";

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/80 p-6 backdrop-blur-sm">
      <div className="w-full max-w-2xl rounded-lg border border-slate-800 bg-slate-900 p-6 shadow-2xl">
        <img src={headerLogo} alt="NHL Modding Studio" className="mx-auto mb-5 h-12 w-auto object-contain" />

        {step === "console" ? (
          <>
            <header className="mb-5 text-center">
              <h1 className="text-xl font-semibold text-slate-100">Select Console</h1>
              <p className="mt-1 text-xs text-slate-400">Which platform are you editing this session?</p>
            </header>
            <div className="space-y-4">
              <div className="mx-auto max-w-xs">
                <ConsoleCard
                  {...CONSOLES[0]}
                  onSelect={() => {
                    setPlatform("recomp");
                    setStep("file");
                  }}
                />
              </div>
              <div className="grid grid-cols-2 gap-4">
                {CONSOLES.slice(1).map((c) => (
                  <ConsoleCard
                    key={c.id}
                    {...c}
                    onSelect={() => {
                      setPlatform(c.id);
                      setStep("file");
                    }}
                  />
                ))}
              </div>
              <div className="grid grid-cols-1">
                <ConsoleCard id="ps4" label="PS4 (coming in a later phase)" image="" disabled onSelect={() => {}} />
              </div>
            </div>
          </>
        ) : (
          <>
            <header className="mb-4 flex items-center justify-between gap-2">
              <h1 className="text-xl font-semibold text-slate-100">Open your EA NHL database</h1>
              <div className="flex items-center gap-2">
                <Pill tone="info">{platform}</Pill>
                <button
                  type="button"
                  onClick={() => setStep("console")}
                  className="text-[11px] text-sky-400 underline-offset-2 hover:underline"
                >
                  ← Change console
                </button>
              </div>
            </header>
            <p className="mb-4 text-xs text-slate-400">
              Pick <code className="text-slate-300">nhlng.db</code> (or a save container). The
              metadata XML beside it and the bundled defs are detected automatically.
            </p>

            <div className="space-y-3">
              <div>
                <label className="mb-1 block text-xs font-medium text-slate-300">Database / save</label>
                <div className="flex gap-2">
                  <input
                    className={input + " flex-1 font-mono text-xs"}
                    value={dbPath}
                    onChange={(e) => setDbPath(e.target.value)}
                    placeholder="e.g. C:/NHL/extracted/db/nhlng.db"
                  />
                  <Button variant="secondary" onClick={() => void browse()}>
                    Browse…
                  </Button>
                </div>
              </div>
              <div className="w-40">
                <label className="mb-1 block text-xs font-medium text-slate-300">Role</label>
                <select className={input} value={role} onChange={(e) => setRole(e.target.value as Role)}>
                  <option value="nhlng">nhlng.db</option>
                  <option value="roster">Roster save</option>
                  <option value="localization">Localization</option>
                </select>
              </div>

              <details className="rounded-md border border-slate-800 bg-slate-950/40 px-3 py-2" open={advanced}>
                <summary
                  className="cursor-pointer text-xs font-medium text-slate-400"
                  onClick={(e) => {
                    e.preventDefault();
                    setAdvanced((a) => !a);
                  }}
                >
                  Advanced — override metadata / defs paths
                </summary>
                {advanced && (
                  <div className="mt-2 space-y-2">
                    <input
                      className={input + " font-mono text-xs"}
                      value={metaXml}
                      onChange={(e) => setMetaXml(e.target.value)}
                      placeholder="metadata XML path (nhlng-meta.xml)"
                    />
                    <input
                      className={input + " font-mono text-xs"}
                      value={defsRoot}
                      onChange={(e) => setDefsRoot(e.target.value)}
                      placeholder="defs root (contains enums/ and relationships/)"
                    />
                  </div>
                )}
              </details>
            </div>

            <div className="mt-5 flex items-center justify-end gap-3">
              <Button
                onClick={() => load.mutate()}
                disabled={load.isPending || !dbPath.trim() || (advanced && (!metaXml.trim() || !defsRoot.trim()))}
              >
                {load.isPending ? "Loading…" : "Load"}
              </Button>
            </div>

            {load.isError && (
              <div className="mt-4">
                <ErrorBanner title="Couldn't load" message={String(load.error)} />
              </div>
            )}
          </>
        )}

        <p className="mt-5 border-t border-slate-800 pt-3 text-center text-[11px] text-amber-300/70">
          Beta — built and verified against NHL Legacy. Other titles are in progress.
        </p>
      </div>
    </div>
  );
}

function ConsoleCard({
  label,
  image,
  recommended,
  disabled,
  onSelect,
}: {
  id: Platform;
  label: string;
  image: string;
  recommended?: boolean;
  disabled?: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onSelect}
      className={
        "group flex w-full flex-col items-center justify-center gap-2 overflow-hidden rounded-lg border p-4 transition-colors " +
        (disabled
          ? "cursor-not-allowed border-slate-800 bg-slate-950/40 opacity-50"
          : "cursor-pointer border-slate-700 bg-slate-950 hover:border-sky-500")
      }
    >
      {image ? (
        <img
          src={image}
          alt={label}
          draggable={false}
          className="h-32 w-full select-none object-contain transition-transform duration-200 ease-out group-hover:scale-105"
        />
      ) : (
        <span className="flex h-32 items-center justify-center text-sm font-semibold text-slate-400">
          {label}
        </span>
      )}
      <span className="text-sm font-semibold text-slate-100">{image ? label : ""}</span>
      {recommended && (
        <span className="rounded-full border border-emerald-600/70 bg-emerald-950/50 px-2.5 py-0.5 text-[10px] font-semibold uppercase tracking-wide text-emerald-300">
          Recommended
        </span>
      )}
    </button>
  );
}

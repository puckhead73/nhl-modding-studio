import { useEffect, useRef, useState } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import type { Platform } from "../lib/client";
import { Button } from "./primitives";

export type GameSource = { kind: "iso" | "dump"; path: string; destination: string };

/** Configure a source only. Disc extraction starts when Project Setup is submitted. */
export function GameSourcePicker({ platform, value, onSelect, onClose }: {
  platform: Platform; value: GameSource | null; onSelect: (source: GameSource) => void; onClose: () => void;
}) {
  const isoSupported = platform === "xbox" || platform === "ps3";
  const [draft, setDraft] = useState<GameSource | null>(value);
  const [step, setStep] = useState<"source" | "destination">("source");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const dialog = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    const element = dialog.current;
    const previousFocus = document.activeElement;
    element?.showModal();
    return () => {
      element?.close();
      if (previousFocus instanceof HTMLElement && previousFocus.isConnected) previousFocus.focus();
    };
  }, []);
  async function pick(kind: "iso" | "dump") {
    setError(""); setBusy(true);
    try {
      const path = await openDialog({ multiple: false, directory: kind === "dump",
        title: kind === "iso" ? "Select game ISO" : "Select dumped ISO root (containing default.xex or the game .bin)",
        ...(kind === "iso" ? { filters: [{ name: "Disc image", extensions: ["iso"] }] } : {}),
      });
      if (typeof path === "string") setDraft({ kind, path, destination: kind === "dump" ? path : "" });
    } catch (e) { setError(String(e)); } finally { setBusy(false); }
  }
  async function pickDestination() {
    setError(""); setBusy(true);
    try {
      const path = await openDialog({ multiple: false, directory: true, title: "Choose parent folder for the extracted game" });
      // Suggest a game-named child; new and empty destinations are both supported.
      if (typeof path === "string") setDraft((d) => d && ({ ...d, destination: path.replace(/[\\/]+$/, "") + "/" + (d.path.split(/[\\/]/).pop()?.replace(/(?:\.dec)?\.iso$/i, "") || "Extracted game") }));
    } catch (e) { setError(String(e)); } finally { setBusy(false); }
  }
  return <dialog ref={dialog} onCancel={(e) => { e.preventDefault(); if (!busy) onClose(); }} aria-labelledby="game-source-title"
    className="fixed inset-0 m-auto h-fit max-h-[calc(100dvh-2rem)] w-[calc(100vw-2rem)] max-w-lg overflow-y-auto rounded-lg border border-slate-700 bg-slate-900 p-6 text-slate-100 backdrop:bg-black/70">
    <h2 id="game-source-title" className="mb-4 text-lg font-semibold">{step === "source" ? "Select Game Source" : "Extract ISO to disk"}</h2>
    {step === "source" ? <div className="space-y-3">
      <div className="flex flex-wrap gap-3">
        <Button disabled={busy || !isoSupported} onClick={() => void pick("iso")}>Game ISO</Button>
        <Button disabled={busy} onClick={() => void pick("dump")}>Dumped ISO folder</Button>
        {(platform === "ps3" || platform === "ps4") && <Button disabled>Game installation folder — coming later</Button>}
      </div>
      {!isoSupported && <p className="text-sm text-slate-400">ISO extraction for this platform will be addressed with its onboarding workflow.</p>}
      {platform === "ps3" && <p className="text-sm text-slate-400">Use a decrypted PS3 ISO. Studio will extract its game files for you.</p>}
      {draft && <p className="break-all text-sm">{draft.kind === "iso" ? "ISO" : "Dumped folder"}: {draft.path}</p>}
    </div> : <div className="space-y-3">
      <p className="text-sm">Modding requires the ISO contents to be extracted to disk. Studio will use this folder as its game source. Mod Manager will choose where your built mods are installed.</p>
      <label className="block text-sm">Extraction destination
        <input aria-label="Extraction destination" className="mt-1 w-full rounded border border-slate-700 bg-slate-950 p-2" value={draft?.destination ?? ""} onChange={(e) => setDraft((d) => d && ({ ...d, destination: e.target.value }))} />
      </label>
      <Button variant="secondary" disabled={busy} onClick={() => void pickDestination()}>Browse extraction location…</Button>
      <p className="text-xs text-slate-400">Use a new or empty folder on an available drive. Cancelled extraction can be retried at the same location. Extraction starts after you click Next on Project Setup.</p>
    </div>}
    {error && <p role="alert" className="mt-3 text-sm text-rose-300">{error}</p>}
    <div className="mt-5 flex justify-end gap-3">
      <Button variant="secondary" disabled={busy} onClick={onClose}>Cancel</Button>
      {step === "destination" && <Button variant="secondary" disabled={busy} onClick={() => setStep("source")}>Back</Button>}
      <Button disabled={busy || !draft || (step === "destination" && !draft.destination.trim())} onClick={() => {
        if (!draft) return;
        if (step === "source" && draft.kind === "iso") setStep("destination");
        else onSelect({ ...draft, destination: draft.destination.trim() });
      }}>Next</Button>
    </div>
  </dialog>;
}

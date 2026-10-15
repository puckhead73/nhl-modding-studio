import { CONSOLE_LABEL, gameById } from "../lib/catalog";
import type { Platform } from "../lib/client";

const PLATFORM_COLOR: Record<Platform, string> = {
  xbox: "text-[#107C10]",
  ps3: "text-purple-400",
  ps4: "text-blue-400",
  ps5: "text-sky-300",
  recomp: "text-slate-300",
};

/** Shared game identity styling for onboarding and Studio views. */
export function GamePlatformPill({ gameId, platform }: { gameId: string; platform: string }) {
  return (
    <span className="inline-flex shrink-0 items-center whitespace-nowrap rounded-full border border-slate-700 bg-slate-950 px-3 py-1 text-xs">
      <span className="text-white">{gameById(gameId)?.label ?? gameId}</span>
      <span className="whitespace-pre text-slate-500"> · </span>
      <span className={PLATFORM_COLOR[platform as Platform] ?? "text-slate-300"}>
        {CONSOLE_LABEL[platform as Platform] ?? platform}
      </span>
    </span>
  );
}

// The in-app portrait-database download / install job: a polled status shared by the shell
// banner, onboarding and Settings. One job runs at a time in the backend; this hook just watches it.
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { call } from "./client";
import type { PortraitInstallStatusDto } from "../bindings/PortraitInstallStatusDto";
import type { PortraitInstallPlanDto } from "../bindings/PortraitInstallPlanDto";

export const PORTRAIT_INSTALL_KEY = ["portraitInstall", "status"] as const;
export const PORTRAIT_PLAN_KEY = ["portraitInstall", "plan"] as const;

export function isActivePhase(phase: string | undefined): boolean {
  return phase === "downloading" || phase === "extracting";
}

export function formatBytes(n: number): string {
  if (n >= 1 << 30) return `${(n / (1 << 30)).toFixed(n >= 10 * (1 << 30) ? 1 : 2)} GiB`;
  if (n >= 1 << 20) return `${(n / (1 << 20)).toFixed(0)} MiB`;
  if (n >= 1 << 10) return `${(n / (1 << 10)).toFixed(0)} KiB`;
  return `${n} B`;
}

/** Progress of the job; polls once a second while a download or extraction is running. */
export function usePortraitInstallStatus(enabled = true) {
  return useQuery({
    queryKey: PORTRAIT_INSTALL_KEY,
    queryFn: () => call("portrait_install_status", {}),
    enabled,
    refetchInterval: (q) => (isActivePhase(q.state.data?.phase) ? 1000 : false),
    staleTime: 500,
  });
}

/** Where the database would go and whether it is already installed. */
export function usePortraitInstallPlan(enabled = true) {
  return useQuery({
    queryKey: PORTRAIT_PLAN_KEY,
    queryFn: () => call("portrait_install_plan", {}),
    enabled,
    staleTime: 10_000,
  });
}

/** Start / cancel actions that refresh the shared status when they return. */
export function usePortraitInstallActions() {
  const qc = useQueryClient();
  const settle = (s: PortraitInstallStatusDto) => {
    qc.setQueryData(PORTRAIT_INSTALL_KEY, s);
    void qc.invalidateQueries({ queryKey: PORTRAIT_PLAN_KEY });
  };
  const download = useMutation({ mutationFn: () => call("portrait_download_start", {}), onSuccess: settle });
  const installArchive = useMutation({
    mutationFn: (path: string) => call("portrait_install_archive", { path }),
    onSuccess: settle,
  });
  const cancel = useMutation({ mutationFn: () => call("portrait_install_cancel", {}), onSuccess: settle });
  return { download, installArchive, cancel };
}

export type { PortraitInstallStatusDto, PortraitInstallPlanDto };

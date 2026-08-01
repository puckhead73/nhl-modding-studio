// The Roster Builder cart — the list of corpus person-seasons queued for import into the open EA
// project, persisted to localStorage so it survives reloads (Phase 5.5). This holds only the user's
// *intent* (which person-season, which target team, create-vs-match, jersey); the actual lowering,
// preview, and commit are backend commands. Populated by "Send to Builder" affordances across the
// Historical browse and edited on the Roster Builder page.

import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from "react";

/** One queued import: a corpus person-season aimed at an EA team, with the create-vs-match decision. */
export type CartItem = {
  /** Stable row id (client-side only). */
  id: string;
  personId: number;
  seasonYear: number;
  /** Human-readable surfaces carried for display so the cart needn't re-query the corpus. */
  personName: string;
  seasonLabel: string;
  position: string | null;
  /** The EA team the import lands on — must be chosen before preview/commit. */
  targetTeamId: number | null;
  /** `null` ⇒ create a new player; a `wBIz` ⇒ update that existing EA player. */
  matchEaPlayerId: number | null;
  /** Optional jersey override. */
  jersey: number | null;
};

/** The shape a "Send to Builder" affordance supplies (the cart assigns the row `id`). */
export type NewCartItem = Omit<CartItem, "id" | "targetTeamId" | "matchEaPlayerId" | "jersey"> &
  Partial<Pick<CartItem, "targetTeamId" | "matchEaPlayerId" | "jersey">>;

const STORAGE_KEY = "nhl-modding-studio.roster-cart.v1";

function load(): CartItem[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? (parsed as CartItem[]) : [];
  } catch {
    return [];
  }
}

function uid(): string {
  try {
    return crypto.randomUUID();
  } catch {
    return `${Date.now()}-${Math.floor(Math.random() * 1e9)}`;
  }
}

type CartCtx = {
  items: CartItem[];
  /** Append an item. Returns `false` (a no-op) when the same (person, season) is already queued. */
  add: (item: NewCartItem) => boolean;
  /** True when this (person, season) is already in the cart. */
  has: (personId: number, seasonYear: number) => boolean;
  update: (id: string, patch: Partial<CartItem>) => void;
  remove: (id: string) => void;
  clear: () => void;
};

const Ctx = createContext<CartCtx | null>(null);

export function RosterCartProvider({ children }: { children: ReactNode }) {
  const [items, setItems] = useState<CartItem[]>(load);

  const commit = useCallback((fn: (s: CartItem[]) => CartItem[]) => {
    setItems((s) => {
      const next = fn(s);
      try {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
      } catch {
        // localStorage unavailable (private mode / quota) — keep the in-memory value.
      }
      return next;
    });
  }, []);

  const value = useMemo<CartCtx>(
    () => ({
      items,
      has: (personId, seasonYear) =>
        items.some((i) => i.personId === personId && i.seasonYear === seasonYear),
      add: (item) => {
        if (items.some((i) => i.personId === item.personId && i.seasonYear === item.seasonYear)) {
          return false;
        }
        commit((s) => [
          ...s,
          {
            targetTeamId: null,
            matchEaPlayerId: null,
            jersey: null,
            ...item,
            id: uid(),
          },
        ]);
        return true;
      },
      update: (id, patch) => commit((s) => s.map((i) => (i.id === id ? { ...i, ...patch } : i))),
      remove: (id) => commit((s) => s.filter((i) => i.id !== id)),
      clear: () => commit(() => []),
    }),
    [items, commit],
  );

  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

export function useRosterCart(): CartCtx {
  const ctx = useContext(Ctx);
  if (!ctx) throw new Error("useRosterCart must be used within a RosterCartProvider");
  return ctx;
}

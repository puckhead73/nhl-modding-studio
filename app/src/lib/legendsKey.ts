// Legends team keys cross two namespaces. The backend keys a team `franchise:<id>` (or a residual
// key like `windsor-legends`), but a colon in a route segment reads as a URL scheme to react-router
// — the same reason the Historical team route uses `~`. These two functions are the only place that
// translation lives, so the list and the detail page can never disagree about it.

/** Backend team key → route segment. */
export function routeKey(teamKey: string): string {
  return teamKey.replace(":", "~");
}

/** Route segment → backend team key. */
export function teamKey(routeSegment: string): string {
  return routeSegment.replace("~", ":");
}

/** The franchise id a team key points at, or null for the residual Legends-only teams. */
export function franchiseIdOf(teamKey: string): number | null {
  if (!teamKey.startsWith("franchise:")) return null;
  const id = Number(teamKey.slice("franchise:".length));
  return Number.isFinite(id) ? id : null;
}

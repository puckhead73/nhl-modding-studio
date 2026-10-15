import { useQueries, useQuery } from "@tanstack/react-query";
import { call } from "./client";
import { positionGroup, ratingKey, type EstimatedRating } from "./rosterSuggestions";

/** Uses the same active formula and cache as the Historical Attributes preview. */
export function useHistoricalRatings(rows: { personId: number; seasonYear: number; position: string | null; isGoalie?: boolean }[]) {
  const profile = useQuery({ queryKey: ["active_formula"], queryFn: () => call("get_active_formula", {}) });
  const results = useQueries({ queries: rows.map(row => ({
    queryKey: ["compute_ratings", row.personId, row.seasonYear, profile.data],
    queryFn: () => call("compute_ratings", { personId: row.personId, seasonYear: row.seasonYear, profile: profile.data! }),
    enabled: !!profile.data,
    staleTime: 60_000,
  })) });
  const ratings = new Map<string, EstimatedRating>();
  rows.forEach((row, i) => ratings.set(ratingKey(row.personId, row.seasonYear), {
    overall: results[i].data?.overall ?? null,
    group: positionGroup(results[i].data?.position ?? row.position, row.isGoalie),
  }));
  return { ratings, ready: profile.isSuccess && !profile.isFetching && results.every(r => r.isSuccess && !r.isFetching),
    error: profile.isError || results.some(r => r.isError) };
}

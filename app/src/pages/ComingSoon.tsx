import { EmptyState } from "../components/primitives";

/// Honest placeholder for a primary section whose surfaces aren't built yet. Keeps the grouped IA
/// real and navigable while later phases fill in the subtabs.
export function ComingSoon({ title, hint }: { title: string; hint: string }) {
  return (
    <div className="mx-auto max-w-2xl py-10">
      <EmptyState title={title} hint={hint} />
    </div>
  );
}

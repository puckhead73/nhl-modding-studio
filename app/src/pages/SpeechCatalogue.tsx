import { useProject } from "../lib/project";
import { SpeechCatalogueCard } from "../components/SpeechCatalogueCard";
import { ContextCatalogueCard } from "../components/ContextCatalogueCard";
import { Ps4SpeechBankCard } from "../components/Ps4SpeechBankCard";
import { Gen7BankCard } from "../components/Gen7BankCard";
import { Gen7SfxBankCard } from "../components/Gen7SfxBankCard";

/// Phase 1.5 / 2.7 / 3.5 — the read-only commentary surfaces of the open title: the gen7 speech
/// catalogue (Xbox 360 / PS3 NHL 11–Legacy) and the context listing, which also covers PS4
/// NHL 15–21, and (24 Sep 2026) the PS4 speech banks with the diagnostic EA-MP3 preview. Each card
/// reports its own coverage refusal when the title is outside it.
export function SpeechCataloguePage() {
  const { project } = useProject();
  const projectId = project!.project_id;
  return (
    <div className="space-y-4">
      <SpeechCatalogueCard projectId={projectId} />
      <ContextCatalogueCard projectId={projectId} />
      <Ps4SpeechBankCard projectId={projectId} />
      <Gen7BankCard projectId={projectId} />
      <Gen7SfxBankCard projectId={projectId} />
    </div>
  );
}

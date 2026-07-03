import type { Evidence, Rule } from "../../generated/bindings";

type EvidencePanelProps = {
  evidence: Evidence[];
  selectedRule: Rule | null;
};

export function EvidencePanel({ evidence, selectedRule }: EvidencePanelProps) {
  const selectedEvidence = selectedRule
    ? evidence.filter((item) => selectedRule.evidence_ids.includes(item.id))
    : [];
  const hasMissingEvidence =
    selectedRule !== null && selectedRule.evidence_ids.some((id) => !evidence.some((item) => item.id === id));

  return (
    <section className="page-panel" aria-label="Evidence panel">
      <h2>Evidence</h2>
      {!selectedRule ? <p>Select a rule to inspect evidence.</p> : null}
      {selectedEvidence.map((item) => (
        <article className="card" key={item.id}>
          <p>Highlighted screenshot: {item.screenshot_id}</p>
          <p>{item.description}</p>
          {item.region ? (
            <p>
              Region: {item.region.x}, {item.region.y}, {item.region.width} × {item.region.height}
            </p>
          ) : null}
        </article>
      ))}
      {hasMissingEvidence ? <p role="alert">Missing evidence</p> : null}
    </section>
  );
}

import { useI18n } from "../../app/i18n";
import type { Evidence, Rule } from "../../generated/bindings";
import { desktop } from "../../lib/desktop";
import type { Screenshot } from "../../lib/desktop";

type EvidencePanelProps = {
  evidence: Evidence[];
  screenshots: Screenshot[];
  selectedRule: Rule | null;
};

function percent(value: number, total: number): string {
  if (total <= 0) {
    return "0%";
  }
  return `${(value / total) * 100}%`;
}

export function EvidencePanel({ evidence, screenshots, selectedRule }: EvidencePanelProps) {
  const { locale } = useI18n();
  const isEnglish = locale === "en-US";
  const selectedEvidence = selectedRule
    ? evidence.filter((item) => selectedRule.evidence_ids.includes(item.id))
    : [];
  const hasMissingEvidence =
    selectedRule !== null && selectedRule.evidence_ids.some((id) => !evidence.some((item) => item.id === id));

  return (
    <section className="evidence-block" aria-label="Evidence panel">
      <h2>{isEnglish ? "Evidence" : "证据"}</h2>
      {!selectedRule ? (
        <p>{isEnglish ? "Select a rule to inspect evidence." : "选择规则后可查看对应证据。"}</p>
      ) : null}
      {selectedEvidence.map((item) => {
        const screenshot =
          screenshots.find((candidate) => candidate.id === item.screenshot_id) ?? null;
        return (
          <article className="card" key={item.id}>
            {screenshot ? (
              <figure style={{ position: "relative", margin: 0 }}>
                <img
                  src={desktop.screenshotUrl(screenshot)}
                  alt={`Evidence screenshot: ${screenshot.page_name}`}
                  style={{ width: "100%", display: "block" }}
                />
                {item.region ? (
                  <span
                    aria-hidden="true"
                    style={{
                      position: "absolute",
                      border: "2px solid #e14b4b",
                      pointerEvents: "none",
                      left: percent(item.region.x, screenshot.width),
                      top: percent(item.region.y, screenshot.height),
                      width: percent(item.region.width, screenshot.width),
                      height: percent(item.region.height, screenshot.height),
                    }}
                  />
                ) : null}
              </figure>
            ) : null}
            <p>Highlighted screenshot: {item.screenshot_id}</p>
            <p>{item.description}</p>
            {item.region ? (
              <p>
                Region: {item.region.x}, {item.region.y}, {item.region.width} × {item.region.height}
              </p>
            ) : null}
          </article>
        );
      })}
      {hasMissingEvidence ? (
        <p role="alert" aria-label="Missing evidence">
          {isEnglish ? "Missing evidence" : "缺少证据"}
        </p>
      ) : null}
    </section>
  );
}

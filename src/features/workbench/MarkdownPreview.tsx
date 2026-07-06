import { useI18n } from "../../app/i18n";
import type { DesignSpec, Rule } from "../../generated/bindings";
import { getAllRules, getRuleGroups } from "./ruleGroups";

type MarkdownPreviewProps = {
  spec: DesignSpec | null;
};

export function MarkdownPreview({ spec }: MarkdownPreviewProps) {
  const { locale } = useI18n();
  const isEnglish = locale === "en-US";
  const markdown = spec ? compilePreviewMarkdown(spec) : "";
  const allRules = spec ? getAllRules(spec) : [];
  const finalized = allRules.filter(
    (rule) => rule.status === "accepted" || rule.status === "edited",
  ).length;

  return (
    <section className="paperwrap" aria-label="Markdown preview">
      <div className="paper-progress">
        <span>{isEnglish ? "DESIGN.md preview" : "DESIGN.md 预览"}</span>
        <span className="mono">
          {isEnglish
            ? `Finalized ${finalized} / ${allRules.length}`
            : `已定稿 ${finalized} / ${allRules.length}`}
        </span>
      </div>
      <div className="paper">
        <pre data-testid="markdown-preview">{markdown}</pre>
      </div>
    </section>
  );
}

function compilePreviewMarkdown(spec: DesignSpec): string {
  const sections = getRuleGroups(spec)
    .map((group) => {
      // Match the backend exporter: only accepted or edited rules reach DESIGN.md.
      const visibleRules = group.rules.filter(
        (rule) => rule.status === "accepted" || rule.status === "edited",
      );
      if (visibleRules.length === 0) {
        return "";
      }

      return [`## ${titleCase(group.key)}`, ...visibleRules.map(formatRule)].join("\n");
    })
    .filter(Boolean);

  return ["# DESIGN.md Preview", ...sections].join("\n\n");
}

function formatRule(rule: Rule): string {
  return `- ${rule.statement}`;
}

function titleCase(value: string): string {
  return `${value.charAt(0).toUpperCase()}${value.slice(1)}`;
}

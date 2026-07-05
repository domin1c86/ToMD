import { useI18n } from "../../app/i18n";
import type { DesignSpec, Rule } from "../../generated/bindings";
import { getRuleGroups } from "./ruleGroups";

type MarkdownPreviewProps = {
  spec: DesignSpec | null;
};

export function MarkdownPreview({ spec }: MarkdownPreviewProps) {
  const { locale } = useI18n();
  const isEnglish = locale === "en-US";
  const markdown = spec ? compilePreviewMarkdown(spec) : "";

  return (
    <section className="page-panel" aria-label="Markdown preview">
      <h2>{isEnglish ? "Markdown preview" : "Markdown 预览"}</h2>
      <pre data-testid="markdown-preview">{markdown}</pre>
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

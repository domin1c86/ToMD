import type { DesignSpec, Rule } from "../../generated/bindings";
import { getRuleGroups } from "./ruleGroups";

type MarkdownPreviewProps = {
  spec: DesignSpec | null;
};

export function MarkdownPreview({ spec }: MarkdownPreviewProps) {
  const markdown = spec ? compilePreviewMarkdown(spec) : "";

  return (
    <section className="page-panel" aria-label="Markdown preview">
      <h2>Markdown preview</h2>
      <pre data-testid="markdown-preview">{markdown}</pre>
    </section>
  );
}

function compilePreviewMarkdown(spec: DesignSpec): string {
  const sections = getRuleGroups(spec)
    .map((group) => {
      const visibleRules = group.rules.filter((rule) => rule.status !== "rejected");
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

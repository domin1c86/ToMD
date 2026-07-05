import { useEffect, useMemo, useState } from "react";
import { Link, useParams } from "react-router-dom";

import { useI18n } from "../../app/i18n";
import type { DesignSpec, Rule, RuleStatus } from "../../generated/bindings";
import { desktop } from "../../lib/desktop";
import type { Screenshot } from "../../lib/desktop";
import { EvidencePanel } from "./EvidencePanel";
import { MarkdownPreview } from "./MarkdownPreview";
import { RuleEditor } from "./RuleEditor";
import { getAllRules, getRuleGroups, replaceRuleInSpec } from "./ruleGroups";

export function WorkbenchPage() {
  const { projectId = "" } = useParams();
  const { locale } = useI18n();
  const isEnglish = locale === "en-US";
  const [spec, setSpec] = useState<DesignSpec | null>(null);
  const [screenshots, setScreenshots] = useState<Screenshot[]>([]);
  const [selectedRuleId, setSelectedRuleId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const rules = useMemo(() => (spec ? getAllRules(spec) : []), [spec]);
  const selectedRule = rules.find((rule) => rule.id === selectedRuleId) ?? rules[0] ?? null;

  useEffect(() => {
    let cancelled = false;

    async function loadSpec() {
      setLoading(true);
      setError(null);
      try {
        const [loadedSpec, loadedScreenshots] = await Promise.all([
          desktop.getDesignSpec({ projectId }),
          desktop.listScreenshots({ projectId }),
        ]);
        if (!cancelled) {
          setSpec(loadedSpec);
          setScreenshots(loadedScreenshots);
          setSelectedRuleId(getAllRules(loadedSpec)[0]?.id ?? null);
        }
      } catch (caught) {
        if (!cancelled) {
          setError(caught instanceof Error ? caught.message : String(caught));
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    }

    void loadSpec();

    return () => {
      cancelled = true;
    };
  }, [projectId]);

  const mutateSelectedRule = async (patch: { statement?: string; status?: RuleStatus }) => {
    if (!spec || !selectedRule) {
      return;
    }

    const previousSpec = spec;
    const optimisticRule: Rule = {
      ...selectedRule,
      statement: patch.statement ?? selectedRule.statement,
      status: patch.status ?? selectedRule.status,
      source: patch.statement ? "user" : selectedRule.source,
    };
    const optimisticSpec = replaceRuleInSpec(spec, optimisticRule);
    setSpec(optimisticSpec);
    setSelectedRuleId(optimisticRule.id);
    setError(null);

    try {
      const persistedSpec = await desktop.updateRule({
        projectId,
        ruleId: selectedRule.id,
        ...patch,
      });
      setSpec(persistedSpec);
      setSelectedRuleId(optimisticRule.id);
    } catch (caught) {
      setSpec(previousSpec);
      setSelectedRuleId(selectedRule.id);
      setError(caught instanceof Error ? caught.message : String(caught));
    }
  };

  return (
    <section>
      <div className="page-header">
        <div>
          <h2>{isEnglish ? "Rule review workbench" : "规则审核工作台"}</h2>
          <p>
            {isEnglish
              ? "Review AI-extracted rules one by one. Only accepted or edited rules go into DESIGN.md."
              : "逐条确认 AI 提取的设计规则。只有 accepted / edited 规则会进入 DESIGN.md。"}
          </p>
          <p>Rule workbench</p>
        </div>
        <Link className="button-primary" to={`/projects/${projectId}/exports`}>
          {isEnglish ? "Export DESIGN.md" : "去导出 DESIGN.md"}
        </Link>
      </div>
      {loading ? <p>Loading design spec…</p> : null}
      {error ? <p role="alert">{error}</p> : null}

      {spec ? (
        <div className="workbench-grid">
          <EvidencePanel evidence={spec.evidence} screenshots={screenshots} selectedRule={selectedRule} />
          <section className="page-panel" aria-label="Rule groups">
            <h2>Rules</h2>
            {getRuleGroups(spec).map((group) => (
              <section key={group.key} aria-label={`${group.key} rules`}>
                <h3>{group.key}</h3>
                {group.rules.map((rule) => (
                  <button
                    className="button-secondary"
                    key={rule.id}
                    type="button"
                    aria-pressed={rule.id === selectedRule?.id}
                    onClick={() => setSelectedRuleId(rule.id)}
                  >
                    {rule.statement}
                  </button>
                ))}
              </section>
            ))}
            <RuleEditor rule={selectedRule} onMutate={mutateSelectedRule} />
          </section>
          <MarkdownPreview spec={spec} />
        </div>
      ) : null}
    </section>
  );
}

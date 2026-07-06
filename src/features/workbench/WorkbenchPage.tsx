import { useEffect, useMemo, useState } from "react";
import { Link, useParams } from "react-router-dom";

import { useI18n } from "../../app/i18n";
import type { DesignSpec, Rule, RuleStatus } from "../../generated/bindings";
import { desktop } from "../../lib/desktop";
import type { Screenshot } from "../../lib/desktop";
import { EvidencePanel } from "./EvidencePanel";
import { FineTuneChat } from "./FineTuneChat";
import { MarkdownPreview } from "./MarkdownPreview";
import { RuleEditor } from "./RuleEditor";
import { getAllRules, getRuleGroups, replaceRuleInSpec } from "./ruleGroups";

function countFinalized(rules: Rule[]): number {
  return rules.filter((rule) => rule.status === "accepted" || rule.status === "edited").length;
}

function statusDotClass(status: RuleStatus): string {
  switch (status) {
    case "accepted":
      return "st st--ok";
    case "edited":
      return "st st--edit";
    case "rejected":
      return "st st--bad";
    default:
      return "st";
  }
}

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
      {loading ? <p>{isEnglish ? "Loading design spec…" : "正在加载设计规范…"}</p> : null}
      {error ? <p role="alert">{error}</p> : null}

      {spec ? (
        <div className="workbench-grid">
          <section className="page-panel rules-rail" aria-label="Rule groups">
            <h2>{isEnglish ? "Rules" : "规则"}</h2>
            {getRuleGroups(spec).map((group) => (
              <section className="rule-group" key={group.key} aria-label={`${group.key} rules`}>
                <p className="rule-group__name">
                  <span>{group.key}</span>
                  <span>{countFinalized(group.rules)}/{group.rules.length}</span>
                </p>
                {group.rules.map((rule) => (
                  <button
                    className="rule-item"
                    key={rule.id}
                    type="button"
                    aria-pressed={rule.id === selectedRule?.id}
                    onClick={() => setSelectedRuleId(rule.id)}
                  >
                    <span className={statusDotClass(rule.status)} aria-hidden="true" />
                    <span>{rule.statement}</span>
                  </button>
                ))}
              </section>
            ))}
          </section>
          <section className="page-panel workbench-center" aria-label="Rule review">
            <RuleEditor rule={selectedRule} onMutate={mutateSelectedRule} />
            <EvidencePanel evidence={spec.evidence} screenshots={screenshots} selectedRule={selectedRule} />
          </section>
          <MarkdownPreview spec={spec} />
        </div>
      ) : null}

      {spec ? (
        <FineTuneChat
          projectId={projectId}
          selectedRuleId={selectedRule?.id ?? null}
          ruleLabel={(ruleId) => {
            const statement = rules.find((rule) => rule.id === ruleId)?.statement ?? ruleId;
            return statement.length > 24 ? `${statement.slice(0, 24)}…` : statement;
          }}
          onApplied={(nextSpec, affectedRuleIds) => {
            setSpec(nextSpec);
            if (affectedRuleIds.length > 0) {
              setSelectedRuleId(affectedRuleIds[0]);
            }
          }}
          onSelectRule={setSelectedRuleId}
        />
      ) : null}
    </section>
  );
}

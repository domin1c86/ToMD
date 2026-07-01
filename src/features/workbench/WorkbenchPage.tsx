import { useEffect, useMemo, useState } from "react";
import { useParams } from "react-router-dom";

import type { DesignSpec, Rule, RuleStatus } from "../../generated/bindings";
import { desktop } from "../../lib/desktop";
import { EvidencePanel } from "./EvidencePanel";
import { MarkdownPreview } from "./MarkdownPreview";
import { RuleEditor } from "./RuleEditor";
import { getAllRules, getRuleGroups, replaceRuleInSpec } from "./ruleGroups";

export function WorkbenchPage() {
  const { projectId = "" } = useParams();
  const [spec, setSpec] = useState<DesignSpec | null>(null);
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
        const loadedSpec = await desktop.getDesignSpec({ projectId });
        if (!cancelled) {
          setSpec(loadedSpec);
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
      <h2>Rule workbench</h2>
      {loading ? <p>Loading design spec…</p> : null}
      {error ? <p role="alert">{error}</p> : null}

      {spec ? (
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "24% 46% 30%",
            gap: "1rem",
            alignItems: "start",
          }}
        >
          <EvidencePanel evidence={spec.evidence} selectedRule={selectedRule} />
          <section aria-label="Rule groups">
            <h2>Rules</h2>
            {getRuleGroups(spec).map((group) => (
              <section key={group.key} aria-label={`${group.key} rules`}>
                <h3>{group.key}</h3>
                {group.rules.map((rule) => (
                  <button
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

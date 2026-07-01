import type { DesignSpec, Rule } from "../../generated/bindings";

export const ruleGroupKeys = [
  "intent",
  "tokens",
  "layout",
  "components",
  "assets",
  "motion",
  "constraints",
] as const;

export type RuleGroupKey = (typeof ruleGroupKeys)[number];

export function getRuleGroups(spec: DesignSpec): Array<{ key: RuleGroupKey; rules: Rule[] }> {
  return ruleGroupKeys.map((key) => ({ key, rules: spec[key] }));
}

export function getAllRules(spec: DesignSpec): Rule[] {
  return getRuleGroups(spec).flatMap((group) => group.rules);
}

export function replaceRuleInSpec(spec: DesignSpec, nextRule: Rule): DesignSpec {
  return {
    ...spec,
    intent: spec.intent.map((rule) => (rule.id === nextRule.id ? nextRule : rule)),
    tokens: spec.tokens.map((rule) => (rule.id === nextRule.id ? nextRule : rule)),
    layout: spec.layout.map((rule) => (rule.id === nextRule.id ? nextRule : rule)),
    components: spec.components.map((rule) => (rule.id === nextRule.id ? nextRule : rule)),
    assets: spec.assets.map((rule) => (rule.id === nextRule.id ? nextRule : rule)),
    motion: spec.motion.map((rule) => (rule.id === nextRule.id ? nextRule : rule)),
    constraints: spec.constraints.map((rule) => (rule.id === nextRule.id ? nextRule : rule)),
  };
}

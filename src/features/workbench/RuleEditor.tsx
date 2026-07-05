import { FormEvent, useEffect, useState } from "react";

import { useI18n } from "../../app/i18n";
import type { Rule, RuleStatus } from "../../generated/bindings";

type RuleEditorProps = {
  rule: Rule | null;
  onMutate: (patch: { statement?: string; status?: RuleStatus }) => Promise<void>;
};

export function RuleEditor({ rule, onMutate }: RuleEditorProps) {
  const { locale } = useI18n();
  const isEnglish = locale === "en-US";
  const [draftStatement, setDraftStatement] = useState("");

  useEffect(() => {
    setDraftStatement(rule?.statement ?? "");
  }, [rule?.id, rule?.statement]);

  if (!rule) {
    return (
      <section aria-label="Selected rule editor">
        <h2>{isEnglish ? "Rule editor" : "规则编辑器"}</h2>
        <p>{isEnglish ? "Select a rule." : "请选择一条规则。"}</p>
      </section>
    );
  }

  const saveEdit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    await onMutate({ statement: draftStatement.trim(), status: "edited" });
  };

  return (
    <section className="card" aria-label="Selected rule editor">
      <h2>{isEnglish ? "Rule editor" : "规则编辑器"}</h2>
      <p>{rule.id}</p>
      <p>Status: {rule.status}</p>
      <p aria-label={`Status badge: ${rule.status}`}>
        <span className={`status-badge status-badge--${rule.status}`}>{rule.status}</span>
      </p>
      <p>Source: {rule.source}</p>
      {rule.confidence < 0.6 ? (
        <p className="alert">{isEnglish ? "Low confidence" : "置信度较低"}</p>
      ) : null}

      <form onSubmit={saveEdit}>
        <label className="field">
          {isEnglish ? "Rule statement" : "规则描述"}
          <textarea
            aria-label="Rule statement"
            value={draftStatement}
            onChange={(event) => setDraftStatement(event.target.value)}
          />
        </label>
        <button className="button-secondary" type="submit" aria-label="Save edit">
          {isEnglish ? "Save edit" : "保存编辑"}
        </button>
      </form>

      <button
        className="button-primary"
        type="button"
        aria-label="Accept rule"
        onClick={() => void onMutate({ status: "accepted" })}
      >
        {isEnglish ? "Accept rule" : "接受规则"}
      </button>
      <button
        className="button-danger"
        type="button"
        aria-label="Reject rule"
        onClick={() => void onMutate({ status: "rejected" })}
      >
        {isEnglish ? "Reject rule" : "驳回规则"}
      </button>
    </section>
  );
}

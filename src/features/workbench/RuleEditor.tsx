import { FormEvent, useEffect, useState } from "react";

import type { Rule, RuleStatus } from "../../generated/bindings";

type RuleEditorProps = {
  rule: Rule | null;
  onMutate: (patch: { statement?: string; status?: RuleStatus }) => Promise<void>;
};

export function RuleEditor({ rule, onMutate }: RuleEditorProps) {
  const [draftStatement, setDraftStatement] = useState("");

  useEffect(() => {
    setDraftStatement(rule?.statement ?? "");
  }, [rule?.id, rule?.statement]);

  if (!rule) {
    return (
      <section aria-label="Selected rule editor">
        <h2>Rule editor</h2>
        <p>Select a rule.</p>
      </section>
    );
  }

  const saveEdit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    await onMutate({ statement: draftStatement.trim(), status: "edited" });
  };

  return (
    <section aria-label="Selected rule editor">
      <h2>Rule editor</h2>
      <p>{rule.id}</p>
      <p>Status: {rule.status}</p>
      <p>Source: {rule.source}</p>
      {rule.confidence < 0.6 ? <p>Low confidence</p> : null}

      <form onSubmit={saveEdit}>
        <label>
          Rule statement
          <textarea
            value={draftStatement}
            onChange={(event) => setDraftStatement(event.target.value)}
          />
        </label>
        <button type="submit">Save edit</button>
      </form>

      <button type="button" onClick={() => void onMutate({ status: "accepted" })}>
        Accept rule
      </button>
      <button type="button" onClick={() => void onMutate({ status: "rejected" })}>
        Reject rule
      </button>
    </section>
  );
}

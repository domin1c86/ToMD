import { FormEvent, useEffect, useRef, useState } from "react";

import { useI18n } from "../../app/i18n";
import type { DesignSpec } from "../../generated/bindings";
import { desktop } from "../../lib/desktop";
import { lastVerifiedProviderId, selectedProviderId } from "../../lib/providerVerification";

const BUBBLE_TTL_MS = 3000;
const MAX_BUBBLES = 2;

type ChatChip = { id: string; label: string };

type ChatMessage = {
  id: number;
  role: "user" | "ai";
  text: string;
  chips?: ChatChip[];
  error?: boolean;
};

type FineTuneChatProps = {
  projectId: string;
  selectedRuleId: string | null;
  ruleLabel: (ruleId: string) => string;
  onApplied: (spec: DesignSpec, affectedRuleIds: string[]) => void;
  onSelectRule: (ruleId: string) => void;
};

export function FineTuneChat({
  projectId,
  selectedRuleId,
  ruleLabel,
  onApplied,
  onSelectRule,
}: FineTuneChatProps) {
  const { locale } = useI18n();
  const isEnglish = locale === "en-US";
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [bubbleIds, setBubbleIds] = useState<number[]>([]);
  const [expanded, setExpanded] = useState(false);
  const [pending, setPending] = useState(false);
  const [instruction, setInstruction] = useState("");
  const [scope, setScope] = useState<"all" | "current">("all");
  const nextId = useRef(1);
  const timers = useRef<number[]>([]);
  const providerId = selectedProviderId() ?? lastVerifiedProviderId();

  useEffect(() => {
    const currentTimers = timers.current;
    return () => {
      currentTimers.forEach((timer) => clearTimeout(timer));
    };
  }, []);

  useEffect(() => {
    if (!expanded) {
      return;
    }
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setExpanded(false);
      }
    };
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [expanded]);

  const dismissBubble = (id: number) => {
    setBubbleIds((current) => current.filter((bubbleId) => bubbleId !== id));
  };

  const pushMessage = (message: Omit<ChatMessage, "id">, showBubble: boolean) => {
    const id = nextId.current;
    nextId.current += 1;
    setMessages((current) => [...current, { id, ...message }]);
    if (showBubble) {
      setBubbleIds((current) => [...current.slice(-(MAX_BUBBLES - 1)), id]);
      const timer = window.setTimeout(() => dismissBubble(id), BUBBLE_TTL_MS);
      timers.current.push(timer);
    }
  };

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const text = instruction.trim();
    if (!text || pending || !providerId) {
      return;
    }

    pushMessage({ role: "user", text }, !expanded);
    setInstruction("");
    setPending(true);
    try {
      const result = await desktop.refineRules({
        projectId,
        providerId,
        instruction: text,
        ruleId: scope === "current" && selectedRuleId ? selectedRuleId : undefined,
      });
      onApplied(result.spec, result.affected_rule_ids);
      pushMessage(
        {
          role: "ai",
          text: isEnglish
            ? `Adjusted ${result.affected_rule_ids.length} rules; all marked as edited for your review:`
            : `已调整 ${result.affected_rule_ids.length} 条规则，均标记为已编辑，等你确认：`,
          chips: result.affected_rule_ids.map((ruleId) => ({
            id: ruleId,
            label: ruleLabel(ruleId),
          })),
        },
        !expanded,
      );
    } catch (caught) {
      const message = caught instanceof Error ? caught.message : String(caught);
      pushMessage(
        {
          role: "ai",
          error: true,
          text: isEnglish ? `Could not apply: ${message}` : `未能应用：${message}`,
        },
        !expanded,
      );
    } finally {
      setPending(false);
    }
  };

  const composer = (
    <form className="chatbar" onSubmit={(event) => void submit(event)} aria-label="Refine rules with an instruction">
      <select
        aria-label="Instruction scope"
        value={scope}
        onChange={(event) => setScope(event.target.value as "all" | "current")}
      >
        <option value="all">{isEnglish ? "All rules" : "全部规则"}</option>
        <option value="current">{isEnglish ? "Current rule" : "当前规则"}</option>
      </select>
      <input
        aria-label="Refine instruction"
        placeholder={isEnglish ? "Type an instruction, e.g. merge duplicate color rules" : "输入修改指令，例如：合并重复的颜色规则"}
        value={instruction}
        disabled={pending || !providerId}
        onChange={(event) => setInstruction(event.target.value)}
      />
      <button
        className="button-primary chatbar__apply"
        type="submit"
        aria-label="Apply instruction"
        disabled={pending || !providerId}
      >
        {pending ? (isEnglish ? "Applying…" : "正在应用…") : isEnglish ? "Apply" : "应用"}
      </button>
    </form>
  );

  const note = (
    <p className="chat-note">
      {providerId
        ? isEnglish
          ? "Instructions and rule text are sent to the configured provider; changes come back as edited, never finalized."
          : "指令与相关规则文本会发送给所配置的 Provider；改动以「已编辑」状态回到列表，不会直接定稿。"
        : isEnglish
          ? "Add an AI model in the settings and pick it on the analysis page to enable refinement."
          : "请先在「设置」中添加模型，并在分析页选择后再使用指令微调。"}
    </p>
  );

  if (expanded) {
    return (
      <div
        className="chat-modal"
        role="dialog"
        aria-modal="true"
        aria-label="Refine chat history"
        onClick={(event) => {
          if (event.target === event.currentTarget) {
            setExpanded(false);
          }
        }}
      >
        <div className="chat-win">
          <header>
            <span className="chat-win__title">{isEnglish ? "Refine chat" : "微调对话"}</span>
            <span className="chat-win__sub">
              {isEnglish ? "Changes return as edited rules" : "改动以「已编辑」状态回到规则列表"}
            </span>
            <button
              className="button-quiet"
              type="button"
              aria-label="Collapse chat"
              onClick={() => setExpanded(false)}
            >
              {isEnglish ? "Collapse" : "收起"}
            </button>
          </header>
          <div className="chat-history">
            {messages.map((message) => (
              <div
                key={message.id}
                className={`chat-msg chat-msg--${message.role}${message.error ? " chat-msg--error" : ""}`}
              >
                <MessageBody message={message} onSelectRule={onSelectRule} />
              </div>
            ))}
          </div>
          {composer}
          {note}
        </div>
      </div>
    );
  }

  return (
    <div className="chatfloat">
      <div className="chat-bubbles" aria-live="polite">
        {bubbleIds.map((id) => {
          const message = messages.find((candidate) => candidate.id === id);
          if (!message) {
            return null;
          }
          return (
            <div
              key={id}
              className={`chat-bubble chat-bubble--${message.role}${message.error ? " chat-msg--error" : ""}`}
            >
              <MessageBody message={message} onSelectRule={onSelectRule} />
              <button
                className="chat-bubble__x"
                type="button"
                aria-label="Dismiss message"
                onClick={() => dismissBubble(id)}
              >
                ✕
              </button>
            </div>
          );
        })}
      </div>
      <div className="chatfloat__bar">
        {composer}
        <button
          className="button-quiet chatfloat__expand"
          type="button"
          aria-label="Expand chat history"
          onClick={() => setExpanded(true)}
        >
          ⤢
        </button>
      </div>
      {note}
    </div>
  );
}

function MessageBody({
  message,
  onSelectRule,
}: {
  message: ChatMessage;
  onSelectRule: (ruleId: string) => void;
}) {
  return (
    <>
      <span>{message.text}</span>
      {message.chips && message.chips.length > 0 ? (
        <span className="chat-chips">
          {message.chips.map((chip) => (
            <button
              key={chip.id}
              className="tag"
              type="button"
              aria-label={`Show rule ${chip.label}`}
              onClick={() => onSelectRule(chip.id)}
            >
              {chip.label}
            </button>
          ))}
        </span>
      ) : null}
    </>
  );
}

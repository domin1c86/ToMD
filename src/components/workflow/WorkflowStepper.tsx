import { Link } from "react-router-dom";

import { useI18n } from "../../app/i18n";

export type WorkflowStep = "project" | "screenshots" | "provider" | "analysis" | "review_export";

type WorkflowStepperProps = {
  activeStep: WorkflowStep;
  projectId: string | null;
};

const workflow: Array<{
  key: WorkflowStep;
  labelKey: "projectStep" | "screenshotsStep" | "providerStep" | "analysisStep" | "reviewExportStep";
  hintKey:
    | "projectStepHint"
    | "screenshotsStepHint"
    | "providerStepHint"
    | "analysisStepHint"
    | "reviewExportStepHint";
}> = [
  { key: "project", labelKey: "projectStep", hintKey: "projectStepHint" },
  { key: "screenshots", labelKey: "screenshotsStep", hintKey: "screenshotsStepHint" },
  { key: "provider", labelKey: "providerStep", hintKey: "providerStepHint" },
  { key: "analysis", labelKey: "analysisStep", hintKey: "analysisStepHint" },
  { key: "review_export", labelKey: "reviewExportStep", hintKey: "reviewExportStepHint" },
];

export function WorkflowStepper({ activeStep, projectId }: WorkflowStepperProps) {
  const { t } = useI18n();
  const activeIndex = workflow.findIndex((step) => step.key === activeStep);

  return (
    <nav className="workflow-stepper" aria-label="Design workflow">
      {workflow.map((step, index) => {
        const state = index < activeIndex ? "done" : index === activeIndex ? "active" : "upcoming";
        const content = (
          <>
            <span className="workflow-stepper__marker">
              {state === "done" ? "✓" : index + 1}
            </span>
            <span>
              <strong>{t(step.labelKey)}</strong>
              <small>{t(step.hintKey)}</small>
            </span>
          </>
        );

        return projectId ? (
          <Link
            className={`workflow-stepper__item workflow-stepper__item--${state}`}
            to={pathForStep(projectId, step.key)}
            key={step.key}
            aria-current={state === "active" ? "step" : undefined}
          >
            {content}
          </Link>
        ) : (
          <span
            className={`workflow-stepper__item workflow-stepper__item--${state}`}
            key={step.key}
            aria-current={state === "active" ? "step" : undefined}
          >
            {content}
          </span>
        );
      })}
    </nav>
  );
}

function pathForStep(projectId: string, step: WorkflowStep): string {
  switch (step) {
    case "project":
    case "screenshots":
      return `/projects/${projectId}`;
    case "provider":
      return `/projects/${projectId}/providers`;
    case "analysis":
      return `/projects/${projectId}/analyze`;
    case "review_export":
      return `/projects/${projectId}/workbench`;
  }
}

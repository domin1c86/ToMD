import type { ComponentPropsWithoutRef, ReactNode } from "react";

type Variant = "primary" | "secondary" | "danger";

type ButtonProps = ComponentPropsWithoutRef<"button"> & {
  variant?: Variant;
};

type LinkButtonProps = ComponentPropsWithoutRef<"a"> & {
  variant?: Variant;
};

type BoxProps = ComponentPropsWithoutRef<"section">;

export function Button({ className, variant = "secondary", ...props }: ButtonProps) {
  return <button className={classNames(buttonClassName(variant), className)} {...props} />;
}

export function LinkButton({ className, variant = "secondary", ...props }: LinkButtonProps) {
  return <a className={classNames(buttonClassName(variant), className)} {...props} />;
}

export function Card({ className, ...props }: BoxProps) {
  return <section className={classNames("card", className)} {...props} />;
}

export function EmptyState({ className, ...props }: BoxProps) {
  return <section className={classNames("empty-state", className)} {...props} />;
}

export function Alert({ className, ...props }: BoxProps) {
  return <section className={classNames("alert", className)} {...props} />;
}

export function HelpPanel({ className, ...props }: BoxProps) {
  return <aside className={classNames("help-panel", className)} {...props} />;
}

export function PageHeader({
  title,
  description,
  action,
}: {
  title: ReactNode;
  description?: ReactNode;
  action?: ReactNode;
}) {
  return (
    <div className="page-header">
      <div>
        <h2>{title}</h2>
        {description ? <p>{description}</p> : null}
      </div>
      {action}
    </div>
  );
}

export function Field({ className, ...props }: ComponentPropsWithoutRef<"label">) {
  return <label className={classNames("field", className)} {...props} />;
}

export function StepIndicator({
  index,
  state = "upcoming",
}: {
  index: number;
  state?: "done" | "active" | "upcoming";
}) {
  return <span className="workflow-stepper__marker">{state === "done" ? "✓" : index}</span>;
}

function buttonClassName(variant: Variant): string {
  if (variant === "primary") {
    return "button-primary";
  }
  if (variant === "danger") {
    return "button-danger";
  }
  return "button-secondary";
}

function classNames(...values: Array<string | undefined>): string {
  return values.filter(Boolean).join(" ");
}

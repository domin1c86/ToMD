# Design intent

- **design intent** (pattern, confidence 0.91): Prefer calm, utility-first surfaces with a clear task hierarchy.

# Visual principles

- **visual principles** (recommendation, confidence 0.88): Use spacing and contrast to separate dense content groups before adding decorative frames.

# Design tokens

- **color** (pattern, confidence 0.94): Use the primary action color only for interactive emphasis. Value: `{"hex":"#2563EB","role":"primary_action"}`
- **spacing** (pattern, confidence 0.90): Use an 8 px spacing rhythm for adjacent controls. Value: `{"base_px":8}`

# Typography

- **typography** (pattern, confidence 0.86): Use a compact type scale with generous line-height for dense data. Value: `{"line_height":1.5,"role":"body","size_px":16}`

# Layout and responsive rules

- **layout** (recommendation, confidence 0.82): Place primary actions in the upper-right of desktop layouts.

# Component conventions

- **cards** (recommendation, confidence 0.91): Use compact card headers with the label before the value.
- **cards** (pattern, confidence 0.84): Use soft containers for grouped metrics without heavy borders.

# Interaction and states

- **states** (recommendation, confidence 0.83): Show state changes through subtle color and shadow shifts.

# Image and icon direction

- **icons** (pattern, confidence 0.80): Prefer simple line icons with rounded stroke endings.

# Do / Don't

- **do** (recommendation, confidence 0.93): Keep primary actions visually distinct from secondary actions.
- **don't** (recommendation, confidence 0.89): Do not use ornamental backgrounds that compete with task content.

# AI implementation checklist

- [ ] Keep primary actions visually distinct from secondary actions.
- [ ] Do not use ornamental backgrounds that compete with task content.

# Diagram Rules

Improve readability without silently redesigning the underlying diagram.

## Scope first

Unless the user explicitly authorizes structural redesign, preserve:

- every business node and its text;
- every relationship and its direction;
- the meaning of shapes and colors.

The first pass is visual: placement, spacing, connector routing, labels, line weight, arrowheads, and contrast. Do not merge nodes, remove paths, invent connections, or replace a process with a different abstraction.

## Decision-node text capacity

- Use a normal diamond with a 10-unit outline fillet for a short decision label.
- When a decision label needs more horizontal reading space, retain the decision silhouette by using a horizontally extended hexagon with its two points at the top and bottom, also with a 10-unit outline fillet. This is the Diagram Rules long-text-decision variant, not a claim of UML or BPMN notation.
- Use the left/right-pointed horizontal hexagon only when its separately documented meaning is intended; do not substitute it for a decision merely because it accommodates text.
- Keep the decision's incoming/outgoing relationships and Yes/No (or equivalent) labels unchanged when applying this visual variant.

## Rectangular nodes

- Apply explicit node-corner values: rectangular content/process/status nodes use `r=4`; diamonds and long-text decision hexagons use a 10-unit outline fillet. This is a node-shape rule, separate from connector-corner smoothing.
- Keep these fixed values for like-for-like nodes within one diagram; preserve the recognisable silhouette of each meaningful notation shape.
- Preserve a capsule/stadium node as a capsule; do not flatten it into a rounded rectangle merely to apply the rectangular corner radius.
- Centre a short, single statement within its node. When node content is a list, left-align the list in rectangles, diamonds, and hexagons, with consistent internal padding.
- Every shape renderer must explicitly provide a rectangular `labelBox`; text layout uses only that box and never a renderer-default text width. For rectangles and capsules, inset the node bounds by 12 units. For diamonds and long-text decision hexagons, define the central `labelBox` when defining the shape; if its content does not fit, enlarge the node rather than infer a wider region at runtime. Centre a single statement; left-align lists within the `labelBox`.
- Use only four fixed `labelBox` size tiers: S = 96×40, M = 200×80, L = 320×120, XL = 480×160. Assign the smallest tier that fits the content; do not stretch a tier. Comparable nodes use the same tier, and a shape's outline adds only its documented fixed padding or points around that box.
- Preserve the diagram's semantic background palette, including meaningful light/dark variants of a hue. Define explicit paired tokens—`background.<role>.subtle` with dark text, `background.<role>.bold` with inverse light text—rather than a universal text colour. Each pair must meet a 4.5:1 contrast ratio for normal text. A middle-tone fill that supports neither pair must be moved to a lighter or darker variant of the same semantic hue; warning/yellow receives its own tested dark-text pairing.

## Execution split: script first

Put deterministic geometry in scripts, not in an AI-only workflow:

- assign ordered source and target ports;
- allocate orthogonal routing tracks;
- when relationships share a visual trunk or rail, keep every relationship as its own complete source-to-target SVG path; make common segments geometrically overlap rather than replacing them with a separate decorative connector;
- minimize crossings before minimizing length;
- avoid unrelated nodes and keep endpoints perpendicular to box faces;
- render every logical 90-degree connector bend with one uniform `r=5` corner radius; preserve straight T-junctions and shared trunks;
- make arrowheads follow the final segment and place the tip exactly on the target edge: never leave a visible gap and never let the marker intrude into the target; use one uniform marker size across the diagram;
- enforce line width, marker size, corner treatment, and contrast;
- detect segment intersections after generation.

An intersection is a failure unless it is a shared endpoint or an explicitly documented shared route. The generated SVG must not retain conflicting default and replacement marker or stroke rules.

Keep connector routing orthogonal: corner smoothing is a fillet on a right-angle route, not a Bézier/spline substitute. The same radius must be used for comparable bends within a diagram; a corner split across multiple SVG segments is still one logical bend and must be smoothed.

## AI's limited role

Use AI for decisions a geometry script cannot establish reliably:

- whether a Diagram Rule applies to this diagram genre;
- whether multiple paths represent the same handoff and may visually share a final trunk;
- whether the task is faithful reproduction or has permission for structural redesign;
- diagnosing exceptions and performing final render-and-look review.

AI does not substitute for deterministic routing or validation.

## Verification

For every transformation:

1. Compare the rendered original and transformed SVG.
2. Check that node count, labels, and relationship count/direction are unchanged unless the user explicitly approved a semantic change.
3. Run geometric checks for unintended crossings, node collisions, marker orientation, endpoint placement, and label/line overlap.
4. Inspect representative rendered regions; report remaining visual defects separately from semantic/design questions.

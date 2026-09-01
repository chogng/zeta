---
description: Zeta browser UI DOM, CSS ownership, accessibility, and rendering rules.
applyTo: "**/src/zeta/**/browser/**/*.ts,**/src/zeta/**/browser/**/*.css,**/src/zeta/**/*.css"
---

# Browser UI Guidelines

Follow [`docs/ui-styling-ownership.md`](../../docs/ui-styling-ownership.md) for detailed ownership rules.

## DOM vocabulary and ownership

- Use `domNode` for the exposed root of a visual component or view whose API is the root DOM itself.
- Use semantic `<role>DomNode` names for owned children and `container` for a caller-provided parent.
- Use `element` for short-lived generic DOM values, tree/model elements, or established generic widget contracts.
- The object that creates stable DOM owns its listener and disposal lifecycle. Register cleanup beside creation.
- A host may size and position a directly hosted component root; it must not mutate or style through the component's internal DOM.

## State projection and CSS

- The component that defines interaction state owns its visual projection.
- Project state through a stable class such as `.checked` alongside the corresponding ARIA attribute. CSS selects the class, not `[aria-*]` attributes.
- Do not use behavior identities such as `data-action-id` as visual selectors.
- Shared colors, typography, elevation, motion, and standard sizing come from semantic design tokens. Tokens provide values; component CSS decides when to use them.
- A legitimate component variation uses a named presentation variant. Do not add host-specific deep selectors or ambiguous boolean styling options.

## Rendering and layout

- Keep presentation in CSS. Inline styles carry computed geometry or component-local custom-property values, not theme branches.
- Keep measurements before mutations to avoid forced layout.
- Do not rebuild stable DOM on every render when retained nodes are cheap and invalidation is explicit.

## Accessibility

- Preserve native semantics, ARIA, keyboard navigation, labels, disabled state, focus visibility, tab relationships, and high-contrast presentation.
- Do not suppress focus outlines without an equivalent visible treatment.

## Learnings

* 对齐上游 UI 时只对齐职责、行为和可观察契约；Zeta 创建并拥有的 DOM 与 CSS 必须使用 Zeta 自己的稳定品牌 class，禁止复制或延续上游产品 root class。

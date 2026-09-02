---
name: ux-theming
description: Apply repository-aware theming, color-token, widget, focus, and high-contrast rules to VS Code or Zeta UI. Use when registering colors or styling themed states; in Zeta, use its existing theme registry and DOM state owners rather than VS Code variables or selectors.
---

This skill covers color registration, CSS variable usage, widget style patterns, focus indicators, and high-contrast theme requirements.

## Repository routing

Determine the target repository before using any literal name or snippet below:

- In `../vscode`, `.monaco-*`, `.hc-*`, `.vscode-high-contrast`, `src/vs`, and `--vscode-*` are literal repository conventions.
- In Zeta, they describe upstream semantics only. Resolve the equivalent through Zeta's existing component root, state owner, and color registry; never paste the snippet or perform a prefix replacement.

### Zeta upstream-alignment guard

When the target is Zeta, the VS Code names and snippets below describe theme semantics only; they are not copyable CSS or DOM templates. Keep Zeta's existing theme owner and generate variables through `zeta-ts/src/zeta/platform/theme/common/colorRegistry.ts`, which emits `--zeta-*`. Never add `--vscode-*`, `.monaco-*`, `.monaco-workbench`, `.hc-black`, `.vscode-high-contrast`, or an upstream DOM/state wrapper merely to reuse a VS Code rule.

Map an upstream color ID to an existing Zeta token when the semantics match. Register a new Zeta token only when the component has a distinct, durable semantic role and provide the theme variants required by Zeta's registry. Theme-name equivalence does not justify copying upstream selector structure, default hex values, focus state ownership, or high-contrast class plumbing.

Before changing Editor or Workbench theming, record the local component root, state classes, focus owner, token owner, and computed-style behavior. Verify the first state-changing slice in a real browser, including focus and high contrast when affected, before migrating more selectors. A prefix replacement from `--vscode-*` to `--zeta-*`, matching screenshots, type checking, and selector-count progress do not prove correct theming.

Concrete rule: `var(--zeta-editor-background)` produced by Zeta's color registry is valid when the component already owns the editor-background semantic. Copying `var(--vscode-editor-background)` or renaming it while also importing `.monaco-editor` nesting is invalid.

Only theme the states required by the current verified component change. A missing upstream stylesheet, color ID, selector, or high-contrast rule does not authorize replacing a Zeta stylesheet, renaming unrelated local states, adding upstream wrappers, or recreating the full VS Code theme surface.

---

## VS Code repository reference

Sections 1–7 below describe literal APIs, selectors, and variables for the VS Code repository. When the target is Zeta, keep only the semantic requirement—such as “use a registered background color” or “focus remains visible in high contrast”—and resolve its concrete token and selector from Zeta source.

## 1. Registering Colors

**File**: `src/vs/platform/theme/common/colorUtils.ts`

```typescript
export const myWidgetBackground = registerColor('myWidget.background',
    { light: '#ffffff', dark: '#252526', hcDark: Color.black, hcLight: Color.white },
    nls.localize('myWidgetBackground', "Background color of My Widget."));
```

**Rules**:
- Provide defaults for all four theme types: `light`, `dark`, `hcDark`, `hcLight`.
- HC themes must use solid colors (avoid transparency) and set explicit borders via `contrastBorder`.
- Use color transforms for derived colors: `transparent()`, `darken()`, `lighten()`, `oneOf()`.
- Reference existing colors when possible instead of hardcoding hex values.

## 2. Color Categories

| File | Colors |
|------|--------|
| `src/vs/platform/theme/common/colors/baseColors.ts` | `foreground`, `focusBorder`, `contrastBorder`, text links |
| `src/vs/platform/theme/common/colors/editorColors.ts` | Editor widgets, find match, errors/warnings |
| `src/vs/platform/theme/common/colors/inputColors.ts` | Input, toggle, validation |
| `src/vs/platform/theme/common/colors/listColors.ts` | List/tree selection, focus, hover, drop |
| `src/vs/platform/theme/common/colors/miscColors.ts` | Badge, scrollbar, progress bar, sash |
| `src/vs/workbench/common/theme.ts` | Tabs, sidebar, status bar, panels, editor groups, banner |

## 3. Using Colors in CSS

The names in this section apply literally only in the VS Code repository. In Zeta, use a `--zeta-*` variable only when it is emitted by Zeta's registry and matches the component's semantic role; never synthesize a variable by changing the prefix.

Colors are injected as CSS custom properties on `.monaco-workbench`:

```
Color ID: editor.background
CSS variable: --vscode-editor-background
Usage: var(--vscode-editor-background)
```

Conversion functions in `colorUtils.ts`:
- `asCssVariable(colorId)` → `'var(--vscode-editor-background)'`
- `asCssVariableName(colorId)` → `'--vscode-editor-background'`

**In CSS files**, reference directly:
```css
.my-widget {
    background-color: var(--vscode-editor-background);
    color: var(--vscode-foreground);
    border: 1px solid var(--vscode-contrastBorder);
}
```

## 4. Widget Styles Pattern

**File**: `src/vs/platform/theme/browser/defaultStyles.ts`

Every widget type has a default style object and an override factory:

```typescript
// Use defaults:
const button = new Button(container, defaultButtonStyles);

// Override specific colors:
const button = new Button(container, getButtonStyles({
    buttonBackground: myCustomBackgroundColor
}));
```

Available defaults: `defaultButtonStyles`, `defaultInputBoxStyles`, `defaultCheckboxStyles`, `defaultToggleStyles`, `defaultDialogStyles`, `defaultListStyles`, `defaultSelectBoxStyles`, `defaultMenuStyles`, `defaultProgressBarStyles`, `defaultCountBadgeStyles`, `defaultBreadcrumbsWidgetStyles`, `defaultKeybindingLabelStyles`, `defaultFindWidgetStyles`.

## 5. Focus Indicators

The selector and variable below are a VS Code repository example. In Zeta, keep the existing focus owner and use the registered Zeta focus token and component state selector; do not introduce `.monaco-*` nesting or `--vscode-focusBorder`.

Defined in `src/vs/workbench/browser/media/style.css`:

```css
.my-widget:focus {
    outline-width: 1px;
    outline-style: solid;
    outline-offset: -1px;
    outline-color: var(--vscode-focusBorder);
}
```

**VS Code repository rules**:
- Use `var(--vscode-focusBorder)` — never hardcode a focus color in VS Code.
- Default `outline-offset: -1px` (inset). Exception: checkboxes use `2px`.
- Active elements suppress focus ring: `.my-widget:active { outline: 0 !important; }`
- Use `.synthetic-focus` class for programmatic focus indication.
- Toggle buttons use `border: 1px dashed var(--vscode-focusBorder)` instead of outline.

### Focus Trapping

Modal dialogs must trap focus within the dialog until dismissed. Use `dom.trackFocus()` and handle `Tab`/`Shift+Tab` cycling.

## 6. High Contrast Theme Rules

The `.hc-black`, `.hc-light`, and `.vscode-high-contrast` selectors below apply literally only in the VS Code repository. In Zeta, inspect and use its existing high-contrast root/state mechanism; do not add those upstream classes or mirror their DOM nesting.

- **Always** provide `hcDark` and `hcLight` defaults when registering colors.
- HC backgrounds: `Color.black` (hcDark), `Color.white` (hcLight).
- HC borders: reference `contrastBorder` — it is `null` in normal themes, visible in HC.
- HC focus: use `activeContrastBorder` (derived from `focusBorder`).
- In CSS, use `.hc-black` / `.hc-light` class selectors for HC-specific overrides:
  ```css
  .hc-black .my-widget { border: 1px solid var(--vscode-contrastBorder); }
  ```
- In TypeScript, check `isHighContrast(theme.type)` for runtime behavior changes.
- **Box shadows** must be removed or replaced in HC mode (shadows are invisible/distracting with high contrast borders):
  ```css
  .my-widget {
      box-shadow: 0 1px 3px var(--vscode-widget-shadow);
  }
  .vscode-high-contrast .my-widget {
      box-shadow: none;
      border: 1px solid var(--vscode-contrastBorder);
  }
  ```

## 7. No Hardcoded Visual Values

The right-hand variables in the table are VS Code repository examples. In Zeta, the same semantic rule means selecting an existing registered `--zeta-*` token; it never means copying `--vscode-*` or inventing a token by prefix substitution.

Reviewers will always flag hardcoded colors, shadows, sizes that should use theme tokens or CSS variables.

| Hardcoded (flagged) | Correct |
|---------------------|---------|
| `rgba(0, 0, 0, 0.12)` | `var(--vscode-widget-shadow)` or theme-aware variable |
| `#252526` | `var(--vscode-editor-background)` |
| `color: white` | `var(--vscode-button-foreground)` |
| `border: 1px solid #ccc` | `var(--vscode-editorWidget-border)` |
| `border: 1px solid …` (width) | `var(--vscode-strokeThickness)` for the 1px width |
| `border-radius: 6px` | `var(--vscode-cornerRadius-medium)` (radius ramp) |
| `padding: 8px 12px` (off-scale) | spacing ramp (`--vscode-spacing-size*`) |
| `font-size: 14px` (arbitrary) | size ramp (`--vscode-fontSize-*`) |
| `font-weight: 500` | `--vscode-fontWeight-semiBold` (no 500) |
| codicon `font-size: 14px` | `--vscode-codiconFontSize` (16) / `-compact` (12) |

**Rule:** If a value relates to color, shadow, or border — it must come from a CSS variable or registered color token. The only exception is `0` (zero) values and purely structural measurements like `100%`.

**Size, spacing, radius, font and stroke** values have their own design-system
**size** tokens (and decision logic — snap maps, the pill→`circle` rule, and the
compact-glyph convention). Those live in the **ux-css-layout** skill (§10
Design-System Size Tokens) and the auto-injected
`.github/instructions/design-tokens.instructions.md`. Reach for those when a flag
is about *how big / how round / how bold* something is rather than *what color*.


---

## Key Files

| Area | File |
|------|------|
| Color registration | `src/vs/platform/theme/common/colorUtils.ts` |
| Color registry (barrel) | `src/vs/platform/theme/common/colorRegistry.ts` |
| Base colors | `src/vs/platform/theme/common/colors/baseColors.ts` |
| Workbench colors | `src/vs/workbench/common/theme.ts` |
| Default widget styles | `src/vs/platform/theme/browser/defaultStyles.ts` |
| Global workbench styles | `src/vs/workbench/browser/media/style.css` |

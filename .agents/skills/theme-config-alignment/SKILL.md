---
name: theme-config-alignment
description: Decide whether theme-related UI changes belong in theme data, the user config.toml [gui] section, or component behavior before implementation. Use when adding or changing theme tokens, palette/style values, typography, visual sizing, animation preferences, or other graphical appearance controls in Zeta.
---

# Theme Config Alignment

Before changing code that affects theme or graphical appearance, identify who should control the value and state the conclusion. Do not add a theme field or hard-coded value before checking whether it is a durable user preference that belongs in `<profile>/config.toml`.

## Choose one owner

| Kind of value | Owner |
| --- | --- |
| Visual identity selected by a theme, such as colors, borders, shadows, state colors, and theme-specific metrics | Theme document/token and `app/theme` resolution |
| Durable graphical preference that should remain the same when switching themes, such as theme selection, editor font family, font size, line height, density, or an accessibility display preference | GUI-owned typed interpretation of the root `[gui]` table |
| Durable terminal preference such as TUI theme selection | TUI-owned typed interpretation of the root `[tui]` table |
| Internal interaction mechanics, such as hover presence, hit testing, capture state, or a timing detail with no user-facing preference | The component or a dedicated interaction/motion policy |

Do not give theme data and config equal ownership of the same value. `[gui]` and `[tui]` are independent; never mirror an appearance value between them or introduce a shared appearance section. `[desktop]` is not an appearance namespace. When config intentionally overrides a theme-derived default, define the precedence at the frontend point that builds the resolved UI style.

## When the value belongs in config.toml

Implement the complete configuration path rather than only accepting a TOML key:

1. Add the typed field, default, and validation only to the frontend that owns the section.
2. Keep `zeta-rs/config` and App Server unaware of field meaning. They may preserve and replace the complete root `[gui]` or `[tui]` table with revision checks, but must not define appearance DTOs or validate frontend fields.
3. Preserve unknown keys when a frontend edits one known key in its own table.
4. Apply the resolved value when that frontend rebuilds theme, typography, measurement, or component styles.
5. Update the relevant config/theme documentation and test frontend validation, TOML round-trip, opaque protocol transport, and the resulting style.

Directory config must not own appearance preferences. GUI, TUI, and other clients interpret only their own root table; model selection and other server-owned settings remain separate fields.

## When the value stays in theme

Use an existing semantic token when it expresses the same role. Add a new token only for a distinct reusable visual role, then update its built-in values, parsing, resolved palette/style mapping, authoring documentation, and focused tests. Do not put pointer state or component lifecycle into theme types.

## Verification

Review the diff for competing defaults or duplicated authorities. Run the smallest config, protocol, theme, component, and application checks that cover the chosen path, and report the ownership decision with the result.

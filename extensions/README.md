# Built-in extensions

This directory contains static extension packages shipped with Zeta during development. Packaging
places the same directory under `zeta-resources/extensions/`.

Except for this README and `BUILD.bazel`, each direct child must be a package directory with a
`package.json`. TextMate packages may put raw JSON or PLIST grammars below the package and reference
them from `contributes.grammars[].path`.
Built-in packages are declarative resources only; this directory is not an extension JavaScript
runtime or a workspace plugin directory.

## Source and distribution boundary

This directory is a runtime input, not a download endpoint. Built-in packages committed here are
copied into `zeta-resources/extensions/` during development and production packaging. Zeta and
Zeterm read those trusted package directories through `zeta-extensions`; the applications do not
authenticate to a Git repository to load built-in extensions.

If the package set later moves to a shared `zeta-extension-packs` repository, that repository may
be private and may own upstream pinning, license/notice preservation, validation, and CI builds.
It must publish versioned extension artifacts for Zeta and Zeterm to consume during their build or
release process. A running application must consume the packaged artifact or extracted extension
directory, never Git credentials or an unversioned repository checkout.

User-installed extensions are a separate profile-level root. They require an explicit registry or
release distribution mechanism and must not be mixed with these built-in resources.

## Bundled packages

The current declarative pack contains the following package directories:

- `css`, `html`, `javascript`, `json`, `markdown-basics`, `python`, `rust`, `shellscript`, `sql`,
  `typescript-basics`, `xml`, and `yaml` provide language IDs, file associations, language
  configuration, TextMate grammars, and—where upstream provides them—snippets.
- `theme-defaults` provides four self-contained VS Code-derived color-theme documents. Themes that
  rely on VS Code `include` composition are intentionally excluded until the packaging pipeline
  can flatten them deterministically.

The manifest is the only source of contribution metadata. `AppServerExtensionService` receives
Rust-validated package resources and projects languages/configuration/snippets/grammars into the
editor language services;
the browser TextMate runtime never imports package files directly. Theme documents are currently
validated and exposed through the extension theme catalog. They are not yet converted into a
selectable complete Workbench `IColorTheme`, so this is a catalog boundary rather than full VS
Code theme activation.

Supported declarative fields are deliberately narrower than a VS Code extension host:

| Contribution | Current state | Owner |
| --- | --- | --- |
| `languages`, file associations, `language-configuration.json` | ✅ loaded and registered | Editor language registry/configuration |
| `snippets` | ✅ parsed and registered as language completion providers | Editor language completion |
| `grammars` | ✅ loaded through Rust resource APIs and TextMate catalog snapshots | Workbench TextMate service |
| `embeddedLanguages`, `tokenTypes`, bracket scope metadata | 部分具备：传给 `vscode-textmate`，`tokenTypes` 投影到 editor token 类型；embedded language 与 bracket 状态尚未成为公共结果字段 | TextMate adapter |
| `themes` | 部分具备：严格解析、资源发现、版本化 catalog | Extension service; Workbench activation remains |
| `configurationDefaults`, `semanticTokenScopes`, extension JavaScript, LSP server declarations | 尚未接入；不会被当成已执行的扩展代码 | 后续 Workbench/Rust language-service 适配 |

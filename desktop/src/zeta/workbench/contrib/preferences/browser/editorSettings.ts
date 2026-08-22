import "./media/editorSettings.css";
import { addDisposableListener, h } from "../../../../base/browser/dom.js";
import type { IContextViewProvider } from "../../../../base/browser/ui/contextview/contextview.js";
import { InputBox } from "../../../../base/browser/ui/inputbox/inputbox.js";
import { SelectBox } from "../../../../base/browser/ui/selectbox/selectbox.js";
import { Switch, Toggle } from "../../../../base/browser/ui/toggle/toggle.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import type { IConfigurationKey, IConfigurationService } from "../../../../platform/configuration/common/configurationService.js";
import { EditorIndentationKind } from "../../../../editor/common/editorIndentation.js";
import { EditorLineWrapping } from "../../../../editor/browser/viewModel/visualLineProjection.js";
import { CodeEditorConfiguration } from "../../codeEditor/common/editorConfiguration.js";
import { WorkspaceSearchConfiguration } from "../../search/common/searchConfiguration.js";
import { EditorSelectionConfiguration } from "../../../common/editorSelectionConfiguration.js";
import { SettingsTree } from "./settingsTree.js";
import { SettingsTreeModel, type SettingsTreeNode } from "./settingsTreeModels.js";

/** Product settings controls for Aster-backed code editors. */
export class EditorSettingsPane extends DisposableOwner {
  readonly element: HTMLDivElement;
  private readonly controls = new Map<string, HTMLInputElement | InputBox | SelectBox | Toggle>();
  private readonly status: HTMLParagraphElement;

  constructor(container: HTMLElement, private readonly configurationService: IConfigurationService, private readonly contextViewProvider: IContextViewProvider) {
    super();
    const ownerDocument = container.ownerDocument;
    this.element = h(ownerDocument, "div");
    this.element.className = "zeta-editor-settings";
    container.append(this.element);

    const note = h(ownerDocument, "p");
    note.className = "zeta-editor-settings-note";
    note.textContent = "Workspace search defaults update immediately. Editor and diff presentation changes apply when that editor is opened.";
    this.element.append(note);

    const model = this.own(new SettingsTreeModel<HTMLElement>());
    model.setChildren([
      this.createGroup("selection", "Editor selection", "Choose what new documents start with while existing resources continue to resolve by content type.", [
        this.createSelectSetting({
          key: EditorSelectionConfiguration.defaultNewDocumentEditor,
          label: "Default editor for new documents",
          description: "Follow the active build mode, or explicitly prefer the Code or Academic editor for new untitled documents.",
          options: [
            { value: "buildMode", label: "Default" },
            { value: "code", label: "Code" },
            { value: "academic", label: "Academic" },
          ],
        }),
        this.createInformationalSetting("Existing resources", "Source files continue to open in the Code Editor, while Academic content types and .zeta-paper files open in the Structured Editor.", "Automatic"),
        this.createInformationalSetting("Editor associations", "Custom glob-to-editor associations are not persisted yet. Use the resource type resolver until the association service is added.", "Not available"),
      ]),
      this.createGroup("typography", "Typography", "Set the typeface and size used for code.", [
        this.createTextSetting(CodeEditorConfiguration.fontFamily, "Font family", "Use a CSS font-family list, or leave this empty to use the default monospace font.", "Default monospace"),
        this.createNumberSetting(CodeEditorConfiguration.fontSize, "Font size", "Set the editor text size in pixels.", 8, 40),
        this.createNumberSetting(CodeEditorConfiguration.lineHeight, "Line height", "Set the height of each editor line in pixels.", 12, 80),
        this.createToggleSetting(CodeEditorConfiguration.fontLigatures, "Font ligatures", "Use programming ligatures when the selected font supports them."),
      ]),
      this.createGroup("display", "Display", "Control how code is presented while you work.", [
        this.createSelectSetting({
          key: CodeEditorConfiguration.wordWrap,
          label: "Word wrap",
          description: "Wrap long lines at the editor viewport instead of scrolling horizontally.",
          options: [
            { value: EditorLineWrapping.Off, label: "Off" },
            { value: EditorLineWrapping.On, label: "On" },
          ],
        }),
        this.createToggleSetting(CodeEditorConfiguration.minimapEnabled, "Minimap", "Show a compact document overview on the right side of the editor."),
        this.createToggleSetting(CodeEditorConfiguration.lineNumbers, "Line numbers", "Show line numbers in the editor gutter."),
        this.createToggleSetting(CodeEditorConfiguration.indentationGuides, "Indentation guides", "Show vertical guides aligned with indentation levels."),
        this.createToggleSetting(CodeEditorConfiguration.bracketPairColorization, "Bracket pair colorization", "Use matching colors to distinguish nested bracket pairs."),
        this.createToggleSetting(CodeEditorConfiguration.stickyScroll, "Sticky scroll", "Keep enclosing scopes visible at the top while scrolling."),
        this.createToggleSetting(CodeEditorConfiguration.highlightActiveLine, "Highlight active line", "Give the line containing the cursor a subtle background highlight."),
        this.createToggleSetting(CodeEditorConfiguration.unicodeHighlights, "Unicode highlights", "Call attention to invisible or easily confused Unicode characters."),
      ]),
      this.createGroup("editing", "Editing", "Choose default editing and formatting behavior.", [
        this.createSelectSetting({
          key: CodeEditorConfiguration.indentationKind,
          label: "Indent using",
          description: "Choose whether indentation inserts spaces or tab characters.",
          options: [
            { value: EditorIndentationKind.Spaces, label: "Spaces" },
            { value: EditorIndentationKind.Tabs, label: "Tabs" },
          ],
        }),
        this.createNumberSetting(CodeEditorConfiguration.tabSize, "Tab size", "Set the number of columns represented by one indentation level.", 1, 32),
        this.createToggleSetting(CodeEditorConfiguration.formatOnSave, "Format on save", "Run the active language formatter before saving a file."),
      ]),
      this.createGroup("code-intelligence", "Code intelligence", "Control language-aware assistance inside code editors.", [
        this.createToggleSetting(CodeEditorConfiguration.suggestions, "Suggestions", "Show completion suggestions from language providers."),
        this.createToggleSetting(CodeEditorConfiguration.inlineCompletions, "Inline completions", "Show provider-supplied completion text directly in the editor."),
        this.createToggleSetting(CodeEditorConfiguration.parameterHints, "Parameter hints", "Show signature information while entering function arguments."),
        this.createToggleSetting(CodeEditorConfiguration.inlayHints, "Inlay hints", "Show inferred types, parameter names, and other inline annotations."),
        this.createToggleSetting(CodeEditorConfiguration.codeLens, "CodeLens", "Show provider actions and references near relevant code."),
      ]),
      this.createGroup("find-and-replace", "Find and replace", "Choose how the editor-local Find widget starts and navigates matches.", [
        this.createToggleSetting(CodeEditorConfiguration.findSeedFromSelection, "Seed from selection", "Use a single-line selection as the initial Find query."),
        this.createToggleSetting(CodeEditorConfiguration.findAutoFindInSelection, "Find in selection automatically", "Limit Find to the current non-empty selection when the widget opens."),
        this.createToggleSetting(CodeEditorConfiguration.findLoop, "Loop through matches", "Wrap from the final match to the first match and back again."),
        this.createToggleSetting(CodeEditorConfiguration.findMatchCase, "Match case by default", "Open Find with case-sensitive matching enabled."),
        this.createToggleSetting(CodeEditorConfiguration.findWholeWord, "Whole word by default", "Open Find with whole-word matching enabled."),
        this.createToggleSetting(CodeEditorConfiguration.findRegularExpression, "Regular expression by default", "Open Find with regular-expression matching enabled."),
      ]),
      this.createGroup("workspace-search", "Workspace search", "Set defaults for searching files across the current workspace.", [
        this.createToggleSetting(WorkspaceSearchConfiguration.matchCase, "Match case", "Start workspace searches in case-sensitive mode."),
        this.createToggleSetting(WorkspaceSearchConfiguration.smartCase, "Smart case", "Use case-sensitive matching automatically when the query contains uppercase characters."),
        this.createToggleSetting(WorkspaceSearchConfiguration.regularExpression, "Use regular expressions", "Interpret workspace search queries as regular expressions by default."),
        this.createTextSetting(WorkspaceSearchConfiguration.includePatterns, "Files to include", "Comma-separated glob patterns included in new workspace searches.", "src/**, packages/**"),
        this.createTextSetting(WorkspaceSearchConfiguration.excludePatterns, "Files to exclude", "Comma-separated glob patterns excluded from new workspace searches.", "**/node_modules/**, **/dist/**"),
        this.createNumberSetting(WorkspaceSearchConfiguration.maxResults, "Maximum results", "Stop a workspace search after this many matches.", 100, 5_000),
      ]),
      this.createGroup("diff-editor", "Diff editor", "Control side-by-side comparison presentation and navigation.", [
        this.createToggleSetting(CodeEditorConfiguration.diffShowLineNumbers, "Line numbers", "Show original and modified line numbers in diff cells."),
        this.createToggleSetting(CodeEditorConfiguration.diffShowInlineChanges, "Inline change highlights", "Highlight the exact changed ranges within modified lines."),
        this.createToggleSetting(CodeEditorConfiguration.diffLoopChanges, "Loop through changes", "Wrap change navigation from the final difference to the first."),
        this.createToggleSetting(CodeEditorConfiguration.diffBreadcrumbs, "Change breadcrumbs", "Show the current change position while navigating a diff."),
      ]),
      this.createGroup("files", "Files", "Apply small consistency fixes when saving code files.", [
        this.createToggleSetting(CodeEditorConfiguration.insertFinalNewLine, "Insert final newline", "Ensure non-empty files end with a line feed when saved."),
      ]),
    ]);
    const tree = this.own(new SettingsTree(this.element, {
      model,
      rootClassName: "zeta-editor-settings-tree",
      groupClassName: "zeta-editor-settings-group",
      groupDescriptionClassName: "zeta-editor-settings-group-description",
      itemsClassName: "zeta-editor-settings-list",
      renderItem: (item) => item.value,
    }));
    this.status = h(ownerDocument, "p");
    this.status.className = "zeta-editor-settings-status";
    this.status.setAttribute("role", "status");
    this.status.setAttribute("aria-live", "polite");
    this.element.append(this.status);
    this.syncControls();
    this.own(configurationService.onDidChangeConfiguration(() => this.syncControls()));
  }

  private createGroup(id: string, title: string, description: string, settings: readonly HTMLElement[]): SettingsTreeNode<HTMLElement> {
    const groupId = `editor.group.${id}`;
    return {
      element: { kind: "group", id: groupId, title, description },
      children: settings.map((setting, index) => this.createTreeItem(groupId, setting, index)),
    };
  }

  private createTreeItem(groupId: string, element: HTMLElement, index: number): SettingsTreeNode<HTMLElement> {
    const configurationKey = element.querySelector<HTMLElement>("[data-configuration-key]")?.dataset.configurationKey;
    const title = element.querySelector(".zeta-editor-setting-title")?.textContent?.trim();
    const description = element.querySelector(".zeta-editor-setting-description")?.textContent?.trim() ?? "";
    if (!title) throw new TypeError(`Editor setting '${configurationKey ?? index}' must have a title`);
    return {
      element: {
        kind: "item",
        id: `${groupId}.item.${configurationKey ?? index}`,
        title,
        description,
        keywords: configurationKey ? [configurationKey] : undefined,
        value: element,
      },
    };
  }

  private createToggleSetting(key: IConfigurationKey<boolean>, label: string, description: string): HTMLElement {
    const document = this.element.ownerDocument;
    const copy = this.createSettingCopy(label, description);
    const host = h(document, "span");
    const toggle = this.own(new Switch(host, {
      ariaLabel: label,
      content: copy,
      contentPlacement: "before-control",
    }));
    toggle.element.classList.add("zeta-editor-setting", "zeta-editor-toggle-setting");
    toggle.input.dataset.configurationKey = key.key;
    this.controls.set(key.key, toggle);
    this.own(toggle.onDidChange(checked => void this.updateConfiguration(key, checked)));
    return toggle.element;
  }

  private createInformationalSetting(label: string, description: string, stateLabel: string): HTMLElement {
    const setting = h(this.element.ownerDocument, "div");
    setting.className = "zeta-editor-setting zeta-editor-informational-setting";
    setting.append(this.createSettingCopy(label, description));
    const state = h(this.element.ownerDocument, "span");
    state.className = "zeta-editor-setting-state";
    state.textContent = stateLabel;
    setting.append(state);
    return setting;
  }

  private createSelectSetting<T extends string>(options: {
    readonly key: IConfigurationKey<T>;
    readonly label: string;
    readonly description: string;
    readonly options: readonly { readonly value: T; readonly label: string }[];
  }): HTMLElement {
    const document = this.element.ownerDocument;
    const setting = h(document, "div");
    setting.className = "zeta-editor-setting zeta-editor-setting-select-row";
    const copy = this.createSettingCopy(options.label, options.description);
    const select = this.own(new SelectBox(setting, {
      options: options.options,
      ariaLabel: options.label,
      presentation: "field",
      contextViewProvider: this.contextViewProvider,
    }));
    select.element.classList.add("zeta-editor-setting-select");
    select.element.dataset.configurationKey = options.key.key;
    setting.append(copy, select.element);
    this.controls.set(options.key.key, select);
    this.own(select.onDidSelect(({ value }) => void this.updateConfiguration(options.key, value as T)));
    return setting;
  }

  private createNumberSetting(key: IConfigurationKey<number>, label: string, description: string, minimum: number, maximum: number): HTMLElement {
    const document = this.element.ownerDocument;
    const setting = h(document, "div");
    setting.className = "zeta-editor-setting";
    const copy = this.createSettingCopy(label, description);
    const input = this.own(new InputBox(setting, {
      type: "number",
      ariaLabel: label,
      presentation: "field",
    }));
    input.element.classList.add("zeta-editor-setting-number");
    input.inputElement.min = String(minimum);
    input.inputElement.max = String(maximum);
    input.step = "1";
    input.inputElement.dataset.configurationKey = key.key;
    setting.append(copy, input.element);
    this.controls.set(key.key, input);
    this.own(addDisposableListener(input.inputElement, "change", () => {
      const value = input.inputElement.valueAsNumber;
      if (!Number.isSafeInteger(value) || value < minimum || value > maximum) {
        this.syncControl(key);
        this.showStatus(`${label} must be between ${minimum} and ${maximum}.`, true);
        return;
      }
      void this.updateConfiguration(key, value);
    }));
    return setting;
  }

  private createTextSetting(key: IConfigurationKey<string>, label: string, description: string, placeholder: string): HTMLElement {
    const document = this.element.ownerDocument;
    const setting = h(document, "label");
    setting.className = "zeta-editor-setting";
    const copy = this.createSettingCopy(label, description);
    const input = h(document, "input");
    input.className = "zeta-editor-setting-text";
    input.type = "text";
    input.placeholder = placeholder;
    input.dataset.configurationKey = key.key;
    setting.append(copy, input);
    this.controls.set(key.key, input);
    this.own(addDisposableListener(input, "change", () => void this.updateConfiguration(key, input.value.trim())));
    return setting;
  }

  private createSettingCopy(label: string, description: string): HTMLElement {
    const document = this.element.ownerDocument;
    const copy = h(document, "span");
    copy.className = "zeta-editor-setting-copy";
    const title = h(document, "span");
    title.className = "zeta-editor-setting-title";
    title.textContent = label;
    const hint = h(document, "span");
    hint.className = "zeta-editor-setting-description";
    hint.textContent = description;
    copy.append(title, hint);
    return copy;
  }

  private syncControls(): void {
    this.syncControl(EditorSelectionConfiguration.defaultNewDocumentEditor);
    this.syncControl(CodeEditorConfiguration.fontFamily);
    this.syncControl(CodeEditorConfiguration.fontSize);
    this.syncControl(CodeEditorConfiguration.lineHeight);
    this.syncControl(CodeEditorConfiguration.fontLigatures);
    this.syncControl(CodeEditorConfiguration.wordWrap);
    this.syncControl(CodeEditorConfiguration.minimapEnabled);
    this.syncControl(CodeEditorConfiguration.lineNumbers);
    this.syncControl(CodeEditorConfiguration.indentationGuides);
    this.syncControl(CodeEditorConfiguration.bracketPairColorization);
    this.syncControl(CodeEditorConfiguration.stickyScroll);
    this.syncControl(CodeEditorConfiguration.highlightActiveLine);
    this.syncControl(CodeEditorConfiguration.unicodeHighlights);
    this.syncControl(CodeEditorConfiguration.indentationKind);
    this.syncControl(CodeEditorConfiguration.tabSize);
    this.syncControl(CodeEditorConfiguration.formatOnSave);
    this.syncControl(CodeEditorConfiguration.suggestions);
    this.syncControl(CodeEditorConfiguration.inlineCompletions);
    this.syncControl(CodeEditorConfiguration.parameterHints);
    this.syncControl(CodeEditorConfiguration.inlayHints);
    this.syncControl(CodeEditorConfiguration.codeLens);
    this.syncControl(CodeEditorConfiguration.findSeedFromSelection);
    this.syncControl(CodeEditorConfiguration.findAutoFindInSelection);
    this.syncControl(CodeEditorConfiguration.findLoop);
    this.syncControl(CodeEditorConfiguration.findMatchCase);
    this.syncControl(CodeEditorConfiguration.findWholeWord);
    this.syncControl(CodeEditorConfiguration.findRegularExpression);
    this.syncControl(WorkspaceSearchConfiguration.matchCase);
    this.syncControl(WorkspaceSearchConfiguration.smartCase);
    this.syncControl(WorkspaceSearchConfiguration.regularExpression);
    this.syncControl(WorkspaceSearchConfiguration.includePatterns);
    this.syncControl(WorkspaceSearchConfiguration.excludePatterns);
    this.syncControl(WorkspaceSearchConfiguration.maxResults);
    this.syncControl(CodeEditorConfiguration.diffShowLineNumbers);
    this.syncControl(CodeEditorConfiguration.diffShowInlineChanges);
    this.syncControl(CodeEditorConfiguration.diffLoopChanges);
    this.syncControl(CodeEditorConfiguration.diffBreadcrumbs);
    this.syncControl(CodeEditorConfiguration.insertFinalNewLine);
  }

  private syncControl<T>(key: IConfigurationKey<T>): void {
    const control = this.controls.get(key.key);
    if (!control) return;
    const value = this.configurationService.getValue(key);
    if (control instanceof Toggle) {
      control.checked = value as boolean;
      return;
    }
    if (control instanceof InputBox) {
      control.value = String(value);
      return;
    }
    if (control instanceof this.element.ownerDocument.defaultView!.HTMLInputElement && control.type === "checkbox") {
      control.checked = value as boolean;
      return;
    }
    control.value = String(value);
  }

  private async updateConfiguration<T>(key: IConfigurationKey<T>, value: T): Promise<void> {
    this.setControlsEnabled(false);
    try {
      await this.configurationService.updateValue(key, value);
      this.showStatus("Setting saved.", false);
    } catch (error) {
      this.syncControl(key);
      this.showStatus(error instanceof Error ? error.message : "Unable to save the setting.", true);
    } finally {
      this.setControlsEnabled(true);
    }
  }

  private setControlsEnabled(enabled: boolean): void {
    this.element.classList.toggle("is-saving", !enabled);
    for (const control of this.controls.values()) {
      if (control instanceof SelectBox) control.enabled = enabled;
      else if (control instanceof Toggle) control.enabled = enabled;
      else if (control instanceof InputBox) control.enabled = enabled;
      else control.disabled = !enabled;
    }
  }

  private showStatus(message: string, error: boolean): void {
    this.status.textContent = message;
    this.status.classList.toggle("is-error", error);
  }
}

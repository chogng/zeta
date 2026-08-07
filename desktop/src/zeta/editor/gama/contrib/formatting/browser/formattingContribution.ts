import type { IContextMenuProvider } from "../../../../../base/browser/contextmenu.js";
import { ToolBar } from "../../../../../base/browser/ui/toolbar/toolbar.js";
import type { IAction } from "../../../../../base/common/actions.js";
import { lxiconsLibrary } from "../../../../../base/common/lxiconsLibrary.js";
import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import type { DocumentTextStyleAttributes, DocumentTextStyleFontFamily } from "../../../common/model/documentSchema.js";

export type FormattingContext = "none" | "text" | "code";

export interface FormattingDocumentAction {
  readonly id: string;
  readonly label: string;
}

export interface FormattingState {
  readonly context: FormattingContext;
  readonly readOnly: boolean;
  readonly bold: boolean;
  readonly italic: boolean;
  readonly fontFamily: DocumentTextStyleFontFamily | undefined;
  readonly fontSize: number | undefined;
  readonly checkedDocumentActionIds: ReadonlySet<string>;
}

export interface FormattingContributionOptions {
  readonly ownerDocument: Document;
  readonly documentActions: readonly FormattingDocumentAction[];
  readonly onToggleMark: (markType: "strong" | "em") => void;
  readonly onSetTextStyle: (attrs: DocumentTextStyleAttributes) => void;
  readonly onClearTextStyle: () => void;
  readonly onRunDocumentAction: (actionId: string) => void;
}

/**
 * Gama's optional persistent document-formatting contribution.
 *
 * The contribution composes the shared ToolBar for runnable commands with
 * native selects for typography values. Its host supplies only formatting
 * state and document-command callbacks, keeping editor lifecycle independent
 * from this Word-like presentation surface.
 */
export class FormattingContribution extends DisposableOwner {
  readonly element: HTMLDivElement;
  private readonly inlineActions: ToolBar;
  private readonly documentActions: ToolBar;
  private readonly typographyControls: HTMLDivElement;
  private readonly codeContext: HTMLDivElement;
  private readonly fontFamily: HTMLSelectElement;
  private readonly fontSize: HTMLSelectElement;

  constructor(private readonly options: FormattingContributionOptions) {
    super();
    const element = options.ownerDocument.createElement("div");
    element.className = "zeta-structured-format-toolbar";
    element.hidden = true;
    element.setAttribute("role", "group");
    element.setAttribute("aria-label", "Gama document formatting");
    this.element = element;
    this.defer(() => element.remove());

    const inlineActions = this.own(new ToolBar({
      contextMenuProvider: emptyFormattingContextMenuProvider,
      ownerDocument: options.ownerDocument,
      ariaLabel: "Text formatting",
      highlightToggledItems: true,
    }));
    inlineActions.element.classList.add("zeta-structured-format-inline-actions");
    inlineActions.element.addEventListener("mousedown", event => event.preventDefault());
    this.inlineActions = inlineActions;

    const typographyControls = options.ownerDocument.createElement("div");
    typographyControls.className = "zeta-structured-format-typography-controls";
    typographyControls.setAttribute("role", "group");
    typographyControls.setAttribute("aria-label", "Font and size");
    this.typographyControls = typographyControls;
    const fontFamily = createSelectControl(options.ownerDocument, "Font", "Font family", [
      { value: "", label: "Default" },
      { value: "sans", label: "Sans serif" },
      { value: "serif", label: "Serif" },
      { value: "monospace", label: "Monospace" },
    ]);
    fontFamily.element.classList.add("zeta-structured-format-font-family");
    fontFamily.select.addEventListener("change", () => {
      const value = fontFamily.select.value;
      if (value === "") {
        options.onClearTextStyle();
        return;
      }
      options.onSetTextStyle({ fontFamily: value as DocumentTextStyleFontFamily });
    });
    this.fontFamily = fontFamily.select;
    const fontSize = createSelectControl(options.ownerDocument, "Size", "Font size", [
      { value: "", label: "Default" },
      ...[10, 11, 12, 14, 16, 18, 20, 24, 28, 32].map(value => ({ value: String(value), label: `${value}` })),
    ]);
    fontSize.element.classList.add("zeta-structured-format-font-size");
    fontSize.select.addEventListener("change", () => {
      const value = fontSize.select.value;
      if (value === "") {
        options.onClearTextStyle();
        return;
      }
      options.onSetTextStyle({ fontSize: Number(value) });
    });
    this.fontSize = fontSize.select;
    typographyControls.append(fontFamily.element, fontSize.element);

    const documentActions = this.own(new ToolBar({
      contextMenuProvider: emptyFormattingContextMenuProvider,
      ownerDocument: options.ownerDocument,
      ariaLabel: "Document structure",
      highlightToggledItems: true,
    }));
    documentActions.element.classList.add("zeta-structured-format-document-actions");
    documentActions.element.addEventListener("mousedown", event => event.preventDefault());
    this.documentActions = documentActions;

    const codeContext = options.ownerDocument.createElement("div");
    codeContext.className = "zeta-structured-format-code-context";
    codeContext.textContent = "Code block · Alpha";
    codeContext.setAttribute("role", "status");
    this.codeContext = codeContext;

    element.append(inlineActions.element, typographyControls, documentActions.element, codeContext);
    this.setState({
      context: "none",
      readOnly: false,
      bold: false,
      italic: false,
      fontFamily: undefined,
      fontSize: undefined,
      checkedDocumentActionIds: new Set(),
    });
  }

  setState(state: FormattingState): void {
    const hasTextContext = state.context === "text";
    this.element.dataset.context = state.context;
    this.inlineActions.element.hidden = !hasTextContext;
    this.typographyControls.hidden = !hasTextContext;
    this.documentActions.element.hidden = !hasTextContext;
    this.codeContext.hidden = state.context !== "code";
    this.fontFamily.disabled = !hasTextContext || state.readOnly;
    this.fontSize.disabled = !hasTextContext || state.readOnly;
    setSelectValue(this.fontFamily, state.fontFamily ?? "");
    setSelectValue(this.fontSize, state.fontSize === undefined ? "" : String(state.fontSize));
    this.inlineActions.setActions([
      createAction("bold", "Bold", "Toggle bold", lxiconsLibrary.bold, hasTextContext && !state.readOnly, state.bold, () => this.options.onToggleMark("strong")),
      createAction("italic", "Italic", "Toggle italic", lxiconsLibrary.italics, hasTextContext && !state.readOnly, state.italic, () => this.options.onToggleMark("em")),
    ]);
    this.documentActions.setActions(this.options.documentActions.map(action => createAction(
      action.id,
      action.label,
      action.label,
      undefined,
      hasTextContext && !state.readOnly,
      state.checkedDocumentActionIds.has(action.id),
      () => this.options.onRunDocumentAction(action.id),
    )));
  }
}

function createAction(id: string, label: string, tooltip: string, icon: IAction["icon"], enabled: boolean, checked: boolean, run: () => void): IAction {
  return { id, label, tooltip, icon, enabled, checked, run };
}

function createSelectControl(ownerDocument: Document, label: string, ariaLabel: string, options: readonly { readonly value: string; readonly label: string }[]): { readonly element: HTMLLabelElement; readonly select: HTMLSelectElement } {
  const element = ownerDocument.createElement("label");
  element.className = "zeta-structured-format-select-control";
  const text = ownerDocument.createElement("span");
  text.className = "zeta-structured-format-select-label";
  text.textContent = label;
  const select = ownerDocument.createElement("select");
  select.className = "zeta-structured-format-select";
  select.setAttribute("aria-label", ariaLabel);
  for (const option of options) {
    const optionElement = ownerDocument.createElement("option");
    optionElement.value = option.value;
    optionElement.textContent = option.label;
    select.append(optionElement);
  }
  element.append(text, select);
  return { element, select };
}

function setSelectValue(select: HTMLSelectElement, value: string): void {
  select.value = value;
  if (select.value !== value) select.value = "";
}

const emptyFormattingContextMenuProvider: IContextMenuProvider = {
  showContextMenu(): never {
    throw new Error("Gama formatting toolbars do not define secondary actions");
  },
};

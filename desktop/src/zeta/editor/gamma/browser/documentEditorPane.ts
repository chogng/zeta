import "./media/documentEditor.css";
import { throwIfCancelled } from "../../../base/common/cancellation.js";
import { DisposableOwner, DisposableSlot, type IDisposable } from "../../../base/common/lifecycle.js";
import { assertDefined } from "../../../base/common/types.js";
import type { IDimension } from "../../../base/browser/geometry.js";
import { URI } from "../../../base/common/uri.js";
import type { ITextFileService } from "../../../workbench/services/textfile/common/textFileService.js";
import { EditorPaneVisibility, type IEditorPane } from "../../../workbench/browser/parts/editor/editorPane.js";
import type { EditorInput } from "../../../workbench/browser/parts/editor/editorInput.js";
import type { IEmbeddedTextEditor, IEmbeddedTextEditorFactory } from "../../../workbench/browser/parts/editor/embeddedTextEditor.js";
import { DocumentModel } from "../common/model.js";
import type { DocumentPlugin } from "../common/plugin.js";
import { containsDocumentNode, findDocumentNode, type DocumentMark, type DocumentNode, type DocumentNodeId } from "../common/document.js";
import type { DocumentDecoration } from "../common/decoration.js";
import { buildDocumentOutline, type DocumentOutline, type DocumentOutlineOptions } from "../common/documentOutline.js";
import { documentPointToPosition } from "../common/documentPosition.js";
import { documentSelectionToText } from "../common/documentText.js";
import { createDeleteAdjacentInlineNodeCommand, createDeleteInlineSelectionCommand, createDeleteNodeSelectionCommand, createDeleteTableColumnCommand, createDeleteTableRowCommand, createExitEmptyListItemCommand, createInsertFragmentCommand, createInsertHardBreakCommand, createInsertHorizontalRuleCommand, createInsertImageAtSelectionCommand, createInsertImageCommand, createInsertParagraphAfterCommand, createInsertTableColumnCommand, createInsertTableCommand, createInsertTableRowCommand, createJoinAdjacentBlockCommand, createJoinAdjacentListItemCommand, createJoinAdjacentTextRunCommand, createListItemIndentationCommand, createMoveBlockCommand, createRemoveMarkCommand, createPasteTextCommand, createReplaceTextCommand, createSetBlockTypeCommand, createSetLinkMarkCommand, createSplitBlockCommand, createSplitListItemCommand, createToggleBlockquoteCommand, createToggleListCommand, createToggleMarkCommand, findAdjacentTableCell, findTableCellContext, type DocumentCommand } from "../common/commands.js";
import { extractDocumentFragment } from "../common/fragment.js";
import { createDefaultDocumentSchema, type DocumentSchema } from "../common/schema.js";
import { DOCUMENT_FRAGMENT_CLIPBOARD_MIME, deserializeDocumentFragment, serializeDocumentFragment } from "../common/serialization.js";
import { allSelection, nodeSelection, textSelection, type DocumentSelection, type TextSelection } from "../common/selection.js";
import { DocumentTransaction } from "../common/transaction.js";
import { DOCUMENT_EDITOR_ID } from "./documentEditorInput.js";
import { DocumentOutlineNavigator } from "./documentOutlineNavigator.js";
import { DocumentWorkingCopy, parseDocument } from "./documentWorkingCopy.js";
import type { IWorkingCopy, IWorkingCopyService } from "../../../workbench/services/workingCopy/common/workingCopyService.js";

export interface DocumentEditorPaneOptions {
  readonly onSave?: () => Promise<void | boolean>;
  readonly embeddedTextEditorFactory?: IEmbeddedTextEditorFactory;
  readonly workingCopyService?: IWorkingCopyService;
  readonly plugins?: readonly DocumentPlugin<unknown>[];
  readonly schema?: DocumentSchema;
  /** Creates the canonical document when the loaded resource has no content. */
  readonly createEmptyDocument?: () => DocumentNode;
  /** Configures the generic heading query exposed by the pane. */
  readonly outline?: DocumentOutlineOptions;
  /** Adds the browser-owned outline navigator to the pane layout. */
  readonly outlineNavigator?: boolean;
  /** Supplies browser projections for profile-owned inline atomic nodes. */
  readonly inlineNodeViews?: Readonly<Record<string, DocumentInlineNodeViewFactory>>;
  /** Adds profile-owned commands to the shared block toolbar. */
  readonly toolbarActions?: readonly DocumentEditorToolbarAction[];
  readonly nodeViews?: Readonly<Record<string, DocumentNodeViewFactory>>;
}

export interface DocumentNodeViewContext {
  readonly node: DocumentNode;
  readonly model: DocumentModel;
  readonly ownerDocument: Document;
  readonly previousElement: HTMLElement | undefined;
  readonly renderChildren: (parent: HTMLElement) => void;
}

export interface DocumentNodeView {
  readonly element: HTMLElement;
  readonly update?: (context: DocumentNodeViewContext) => boolean;
  readonly dispose?: () => void;
}

export type DocumentNodeViewFactory = (context: DocumentNodeViewContext) => HTMLElement | DocumentNodeView;

export interface DocumentInlineNodeViewContext {
  readonly node: DocumentNode;
  readonly model: DocumentModel;
  readonly ownerDocument: Document;
  readonly select: () => void;
}

export type DocumentInlineNodeViewFactory = (context: DocumentInlineNodeViewContext) => HTMLElement;

export interface DocumentEditorToolbarActionContext {
  readonly model: DocumentModel;
  readonly blockId: DocumentNodeId;
  readonly selection: TextSelection | undefined;
  readonly ownerDocument: Document;
}

export interface DocumentEditorToolbarAction {
  readonly id: string;
  readonly label: string;
  readonly run: (context: DocumentEditorToolbarActionContext) => DocumentCommand | undefined;
}

/** Browser host for Gamma's structured document model. */
export class DocumentEditorPane extends DisposableOwner implements IEditorPane {
  readonly id = DOCUMENT_EDITOR_ID;

  private readonly modelSlot = this.own(new DisposableSlot<DocumentModel>());
  private readonly modelChangeListenerSlot = this.own(new DisposableSlot<IDisposable>());
  private readonly workingCopySlot = this.own(new DisposableSlot<IWorkingCopy>());
  private readonly schema: DocumentSchema;
  private readonly embeddedEditors = new Map<string, IEmbeddedTextEditor>();
  private readonly nodeViewSlots = new Map<string, { readonly type: string; readonly view: DocumentNodeView }>();
  private container: HTMLDivElement | undefined;
  private layoutContainer: HTMLDivElement | undefined;
  private toolbar: HTMLDivElement | undefined;
  private outlineNavigator: DocumentOutlineNavigator | undefined;
  private input: EditorInput | undefined;
  private activeBlockId: string | undefined;
  private composition: DocumentComposition | undefined;
  private dimension: IDimension = { width: 0, height: 0 };

  get workingCopy(): IWorkingCopy | undefined {
    return this.workingCopySlot.value;
  }

  constructor(private readonly textFiles: ITextFileService, private readonly options: DocumentEditorPaneOptions = {}) {
    super();
    if (!textFiles || typeof textFiles.resolve !== "function" || typeof textFiles.save !== "function") {
      this.dispose();
      throw new TypeError("Document editor pane requires a Workbench text file service");
    }
    this.schema = options.schema ?? createDefaultDocumentSchema();
    this.defer(() => this.disposeEmbeddedEditors());
    this.defer(() => this.disposeNodeViews());
  }

  create(parent: HTMLElement): void {
    if (this.container) throw new ReferenceError("DocumentEditorPane has already been created");
    const toolbar = parent.ownerDocument.createElement("div");
    toolbar.className = "zeta-document-block-toolbar";
    toolbar.hidden = true;
    toolbar.setAttribute("role", "toolbar");
    toolbar.setAttribute("aria-label", "Block formatting");
    const toolbarActions = [{ type: "paragraph", label: "Paragraph" }, { type: "heading", label: "Heading" }, { type: "blockquote", label: "Blockquote" }, { type: "bulletList", label: "Bullet list" }, { type: "orderedList", label: "Ordered list" }, { type: "horizontalRule", label: "Rule" }, { type: "link", label: "Link" }, { type: "unlink", label: "Unlink" }, { type: "table", label: "Table" }, { type: "insertTableRow", label: "Add row" }, { type: "insertTableColumn", label: "Add column" }, { type: "deleteTableRow", label: "Delete row" }, { type: "deleteTableColumn", label: "Delete column" }, ...(this.options.toolbarActions?.map(action => ({ type: action.id, label: action.label })) ?? [])];
    for (const action of toolbarActions) {
      const button = parent.ownerDocument.createElement("button");
      button.type = "button";
      button.className = "zeta-document-block-toolbar-button";
      button.dataset.blockType = action.type;
      button.textContent = action.label;
      button.addEventListener("mousedown", event => event.preventDefault());
      button.addEventListener("click", () => this.handleToolbarAction(action.type));
      toolbar.append(button);
    }
    const container = parent.ownerDocument.createElement("div");
    container.className = "zeta-document-editor-pane";
    const layoutContainer = parent.ownerDocument.createElement("div");
    layoutContainer.className = "zeta-document-editor-layout";
    const outlineNavigator = this.options.outlineNavigator ? new DocumentOutlineNavigator({ ownerDocument: parent.ownerDocument, onSelect: nodeId => this.revealOutlineNode(nodeId) }) : undefined;
    if (outlineNavigator) layoutContainer.append(outlineNavigator.element);
    layoutContainer.append(container);
    parent.append(toolbar, layoutContainer);
    this.toolbar = toolbar;
    this.container = container;
    this.layoutContainer = layoutContainer;
    this.outlineNavigator = outlineNavigator;
    const onSelectionChange = () => this.syncDocumentSelection();
    parent.ownerDocument.addEventListener("selectionchange", onSelectionChange);
    this.defer(() => {
      parent.ownerDocument.removeEventListener("selectionchange", onSelectionChange);
      outlineNavigator?.dispose();
      toolbar.remove();
      this.toolbar = undefined;
      this.layoutContainer = undefined;
      container.remove();
      this.outlineNavigator = undefined;
      this.container = undefined;
    });
  }

  async setInput(input: EditorInput, signal: AbortSignal): Promise<void> {
    const container = this.requireContainer();
    throwIfCancelled(signal, "Document editor input loading was cancelled");
    const content = await this.textFiles.resolve({
      resource: input.resource,
      ...(input.initialText === undefined ? {} : { bootstrapText: input.initialText }),
    }, signal);
    throwIfCancelled(signal, "Document editor input loading was cancelled");
    const document = parseDocument(content.text, this.schema, this.options.createEmptyDocument);
    const model = new DocumentModel(this.schema, document, { plugins: this.options.plugins });
    const workingCopy = new DocumentWorkingCopy({
      resource: input.resource,
      model,
      initialDocument: document,
      textFiles: this.textFiles,
      workingCopyService: this.options.workingCopyService,
      onSave: this.options.onSave,
      createEmptyDocument: this.options.createEmptyDocument,
    });
    this.modelChangeListenerSlot.clear();
    this.workingCopySlot.clear();
    this.modelSlot.replace(model);
    this.modelChangeListenerSlot.replace(model.onDidChange(() => this.render()));
    this.workingCopySlot.replace(workingCopy);
    this.input = input;
    this.activeBlockId = undefined;
    this.disposeEmbeddedEditors();
    container.replaceChildren();
    if (this.toolbar) this.toolbar.hidden = false;
    this.render();
  }

  clearInput(): void {
    this.composition = undefined;
    this.modelChangeListenerSlot.clear();
    this.workingCopySlot.clear();
    this.modelSlot.clear();
    this.disposeEmbeddedEditors();
    this.disposeNodeViews();
    this.input = undefined;
    this.activeBlockId = undefined;
    this.outlineNavigator?.setOutline([]);
    if (this.toolbar) this.toolbar.hidden = true;
    this.container?.replaceChildren();
  }

  layout(dimension: IDimension): void {
    this.dimension = { width: Math.max(0, dimension.width), height: Math.max(0, dimension.height) };
    if (this.layoutContainer) {
      this.layoutContainer.style.width = `${this.dimension.width}px`;
      this.layoutContainer.style.height = `${this.dimension.height}px`;
    }
    if (this.container) {
      this.container.style.width = `${this.dimension.width}px`;
      this.container.style.height = `${this.dimension.height}px`;
      for (const editor of this.embeddedEditors.values()) editor.layout(this.embeddedEditorDimension());
    }
  }

  setVisible(visibility: EditorPaneVisibility): void {
    if (this.container) this.container.hidden = visibility === EditorPaneVisibility.Hidden;
  }

  focus(): void {
    const firstEmbeddedEditor = this.embeddedEditors.values().next().value;
    if (firstEmbeddedEditor) {
      firstEmbeddedEditor.focus();
      return;
    }
    this.container?.querySelector<HTMLTextAreaElement>("textarea")?.focus();
  }

  async save(): Promise<void> {
    await this.requireWorkingCopy().save(new AbortController().signal);
  }

  async saveAs(resource: URI): Promise<void> {
    await this.requireWorkingCopy().saveAs(resource, new AbortController().signal);
  }

  async revert(): Promise<void> {
    await this.requireWorkingCopy().revert(new AbortController().signal);
  }

  get isDirty(): boolean {
    return this.workingCopy?.isDirty ?? false;
  }

  getDocument(): DocumentNode {
    return this.requireModel().document;
  }

  getOutline(): DocumentOutline {
    return buildDocumentOutline(this.requireModel().document, this.options.outline);
  }

  private revealOutlineNode(nodeId: string): void {
    const model = this.modelSlot.value;
    const container = this.container;
    if (!model || !container) return;
    const target = Array.from(container.querySelectorAll<HTMLElement>("[data-node-id]")).find(element => element.dataset.nodeId === nodeId);
    if (!target) return;
    target.scrollIntoView?.({ block: "nearest" });
    const node = findNode(model.document, nodeId);
    if (node?.type === "paragraph" || node?.type === "heading") {
      this.focusBlockAtBoundary(model, nodeId, "forward");
      return;
    }
    target.focus();
  }

  private render(): void {
    const container = this.requireContainer();
    const model = this.modelSlot.value;
    if (!model) return;
    const previousElements = new Map<string, HTMLElement>();
    for (const element of container.querySelectorAll<HTMLElement>("[data-node-id]")) {
      if (element.dataset.nodeId) previousElements.set(element.dataset.nodeId, element);
    }
    const activeNodeIds = new Set<string>();
    const decorations = resolveViewDecorations(model);
    const fragment = container.ownerDocument.createDocumentFragment();
    for (const node of model.document.content) fragment.append(this.renderNode(node, model, previousElements, activeNodeIds, decorations));
    container.replaceChildren(fragment);
    this.outlineNavigator?.setOutline(this.getOutline());
    for (const [nodeId, editor] of this.embeddedEditors) {
      if (activeNodeIds.has(nodeId)) continue;
      editor.dispose();
      this.embeddedEditors.delete(nodeId);
    }
    for (const [nodeId, slot] of this.nodeViewSlots) {
      if (activeNodeIds.has(nodeId)) continue;
      slot.view.dispose?.();
      this.nodeViewSlots.delete(nodeId);
    }
    this.updateToolbar();
    this.updateInlineNodeSelection();
  }

  private disposeEmbeddedEditors(): void {
    for (const editor of this.embeddedEditors.values()) editor.dispose();
    this.embeddedEditors.clear();
  }

  private disposeNodeViews(): void {
    for (const slot of this.nodeViewSlots.values()) slot.view.dispose?.();
    this.nodeViewSlots.clear();
  }

  private renderNode(node: DocumentNode, model: DocumentModel, previousElements: Map<string, HTMLElement>, activeNodeIds: Set<string>, decorations: readonly ViewDecoration[]): HTMLElement {
    const document = this.requireContainer().ownerDocument;
    activeNodeIds.add(node.id);
    const nodeView = this.options.nodeViews?.[node.type];
    if (nodeView) {
      const context = { node, model, ownerDocument: document, previousElement: this.nodeViewSlots.get(node.id)?.view.element ?? previousElements.get(node.id), renderChildren: (parent: HTMLElement) => this.renderChildren(parent, node, model, previousElements, activeNodeIds, decorations) };
      const existing = this.nodeViewSlots.get(node.id);
      if (existing && existing.type !== node.type) {
        existing.view.dispose?.();
        this.nodeViewSlots.delete(node.id);
      }
      const current = this.nodeViewSlots.get(node.id);
      if (current?.view.update && !current.view.update(context)) {
        current.view.dispose?.();
        this.nodeViewSlots.delete(node.id);
      }
      const refreshed = this.nodeViewSlots.get(node.id);
      if (refreshed) {
        refreshed.view.element.dataset.nodeId = node.id;
        return refreshed.view.element;
      }
      const created = normalizeDocumentNodeView(nodeView(context), node.type);
      if (created.dispose || created.update) this.nodeViewSlots.set(node.id, { type: node.type, view: created });
      created.element.dataset.nodeId = node.id;
      return created.element;
    }
    if (node.type === "horizontalRule") {
      const rule = this.reuseElement(node, previousElements, document, "hr");
      rule.dataset.nodeId = node.id;
      rule.className = "zeta-document-horizontal-rule";
      return rule;
    }
    const element = this.reuseElement(node, previousElements, document, nodeElementTagName(node));
    element.dataset.nodeId = node.id;
    switch (node.type) {
      case "paragraph":
        element.className = "zeta-document-paragraph";
        this.appendEditableText(element, node, model, decorations);
        break;
      case "heading":
        element.className = "zeta-document-heading";
        element.dataset.level = String(node.attrs.level ?? 1);
        this.appendEditableText(element, node, model, decorations);
        break;
      case "codeBlock":
        element.className = "zeta-document-code-block";
        element.dataset.editorKind = "code-block";
        this.appendCodeBlockEditor(element, node, model, decorations);
        break;
      case "blockquote":
        element.className = "zeta-document-blockquote";
        this.renderChildren(element, node, model, previousElements, activeNodeIds, decorations);
        break;
      case "bulletList":
      case "orderedList":
        element.className = node.type === "bulletList" ? "zeta-document-list zeta-document-list-bullet" : "zeta-document-list zeta-document-list-ordered";
        this.renderChildren(element, node, model, previousElements, activeNodeIds, decorations);
        break;
      case "listItem":
        element.className = "zeta-document-list-item";
        this.renderChildren(element, node, model, previousElements, activeNodeIds, decorations);
        break;
      case "table":
        element.className = "zeta-document-table";
        this.renderChildren(element, node, model, previousElements, activeNodeIds, decorations);
        break;
      case "tableRow":
        element.className = "zeta-document-table-row";
        this.renderChildren(element, node, model, previousElements, activeNodeIds, decorations);
        break;
      case "tableCell":
        element.className = "zeta-document-table-cell";
        this.renderChildren(element, node, model, previousElements, activeNodeIds, decorations);
        break;
      default:
        element.className = "zeta-document-block";
        this.renderChildren(element, node, model, previousElements, activeNodeIds, decorations);
        break;
    }
    return element;
  }

  private appendCodeBlockEditor(element: HTMLElement, node: DocumentNode, model: DocumentModel, decorations: readonly ViewDecoration[]): void {
    const textNode = node.content.find(child => child.text !== undefined);
    const text = textNode?.text ?? "";
    const factory = this.options.embeddedTextEditorFactory;
    if (!factory) {
      this.appendEditableText(element, node, model, decorations);
      return;
    }
    const existingEditor = this.embeddedEditors.get(node.id);
    if (existingEditor) {
      existingEditor.setValue(text);
      existingEditor.layout(this.embeddedEditorDimension());
      return;
    }
    element.replaceChildren();
    const editor = factory.create({
      resource: URI.parse(`untitled:gamma-code-block/${encodeURIComponent(this.requireInput().resource.toString())}/${encodeURIComponent(node.id)}`),
      label: `${this.requireInput().label ?? "Document"} code block`,
      languageId: typeof node.attrs.language === "string" ? node.attrs.language : "plaintext",
      initialText: text,
      readOnly: this.requireInput().readOnly,
    });
    this.embeddedEditors.set(node.id, editor);
    editor.onDidChange(value => {
      const currentModel = this.modelSlot.value;
      if (currentModel !== model) return;
      const currentNode = findNode(currentModel.document, node.id);
      if (!currentNode) return;
      const currentText = currentNode.content.find(child => child.text !== undefined);
      if (currentText?.text === value) return;
      if (!currentText) {
        if (value.length === 0) return;
        currentModel.dispatch(new DocumentTransaction().insertNode(node.id, 0, currentModel.schema.createText(value, { id: `${node.id}-text` })).withHistoryGroup("typing"));
        return;
      }
      currentModel.dispatch(new DocumentTransaction().replaceText(currentText.id, 0, currentText.text?.length ?? 0, value).withHistoryGroup("typing"));
    });
    editor.create(element);
    editor.layout(this.embeddedEditorDimension());
  }

  private embeddedEditorDimension(): IDimension {
    return {
      width: Math.max(0, this.dimension.width),
      height: Math.max(120, this.dimension.height),
    };
  }

  private appendEditableText(element: HTMLElement, node: DocumentNode, model: DocumentModel, decorations: readonly ViewDecoration[]): void {
    if (usesRichTextSurface(node) || hasRenderableDecoration(model, node, decorations)) {
      this.appendRichText(element, node, model, decorations);
      return;
    }
    const textNode = node.content.find(child => child.text !== undefined);
    let textarea = element.querySelector<HTMLTextAreaElement>("textarea.zeta-document-text-input");
    if (!textarea && element.childElementCount > 0) {
      this.embeddedEditors.get(node.id)?.dispose();
      this.embeddedEditors.delete(node.id);
      element.replaceChildren();
    }
    const isNew = !textarea;
    textarea ??= element.ownerDocument.createElement("textarea");
    textarea.className = "zeta-document-text-input";
    textarea.dataset.blockId = node.id;
    textarea.readOnly = this.isReadOnly();
    textarea.setAttribute("aria-readonly", String(textarea.readOnly));
    const nextValue = textNode?.text ?? "";
    if (textarea.value !== nextValue) textarea.value = nextValue;
    textarea.rows = Math.max(1, textarea.value.split("\n").length);
    if (!isNew) return;
    textarea.addEventListener("focus", () => {
      this.activeBlockId = node.id;
      this.updateToolbar();
    });
    const syncTextareaSelection = () => {
      if (this.modelSlot.value !== model) return;
      const start = textarea.selectionStart ?? 0;
      const end = textarea.selectionEnd ?? start;
      if (model.selection?.kind === "all" && start === 0 && end === textarea.value.length) return;
      const selection = createTextareaTextSelection(model.document, node.id, textarea);
      if (selection) model.setSelection(selection);
    };
    textarea.addEventListener("select", syncTextareaSelection);
    textarea.addEventListener("mouseup", syncTextareaSelection);
    textarea.addEventListener("keydown", event => this.handleTextKeydown(event, node, model, textarea));
    textarea.addEventListener("paste", event => this.handleTextPaste(event, model, node.id, textarea));
    textarea.addEventListener("copy", event => this.handleTextClipboard(event, model, node.id, textarea, false));
    textarea.addEventListener("cut", event => this.handleTextClipboard(event, model, node.id, textarea, true));
    textarea.addEventListener("compositionstart", () => this.beginComposition(model, node.id, textarea, createTextareaTextSelection(model.document, node.id, textarea)));
    textarea.addEventListener("compositionend", event => this.endComposition(event, textarea));
    textarea.addEventListener("compositioncancel", () => this.cancelComposition(textarea));
    textarea.addEventListener("input", () => {
      const currentModel = this.modelSlot.value;
      if (currentModel !== model) return;
      if (this.isReadOnly()) return;
      if (this.composition?.element === textarea) return;
      if (currentModel.selection?.kind === "all") {
        const command = createReplaceTextCommand(currentModel.schema, currentModel.document, node.id, currentModel.selection, textarea.value, documentInsertionMarks(currentModel, currentModel.selection));
        if (command) this.dispatchCommand(currentModel, command, "typing");
        return;
      }
      const currentNode = findNode(currentModel.document, node.id);
      if (!currentNode) return;
      const currentText = currentNode.content.find(child => child.text !== undefined);
      const selectionStart = textarea.selectionStart ?? textarea.value.length;
      const selectionEnd = textarea.selectionEnd ?? selectionStart;
      if (!currentText) {
        if (textarea.value.length === 0) return;
        const inserted = currentModel.schema.createText(textarea.value, { id: `${node.id}-text`, marks: documentInsertionMarks(currentModel, currentModel.selection) ?? [] });
        currentModel.dispatch(new DocumentTransaction()
          .insertNode(node.id, 0, inserted)
          .withSelection(textSelection({ nodeId: inserted.id, offset: selectionEnd }))
          .withHistoryGroup("typing"));
      } else {
        let transaction = new DocumentTransaction().replaceText(currentText.id, 0, currentText.text?.length ?? 0, textarea.value, documentInsertionMarks(currentModel, currentModel.selection));
        if (textarea.value.length > 0) transaction = transaction.withSelection(textSelection({ nodeId: currentText.id, offset: selectionEnd }));
        currentModel.dispatch(transaction.withHistoryGroup("typing"));
      }
      const nextTextarea = findTextArea(this.requireContainer(), node.id);
      nextTextarea?.focus();
      if (nextTextarea) {
        nextTextarea.setSelectionRange(Math.min(selectionStart, nextTextarea.value.length), Math.min(selectionEnd, nextTextarea.value.length));
      }
    });
    element.append(textarea);
  }

  private appendRichText(element: HTMLElement, node: DocumentNode, model: DocumentModel, decorations: readonly ViewDecoration[]): void {
    let editor = element.querySelector<HTMLDivElement>("div.zeta-document-rich-text-input");
    if (!editor) {
      this.embeddedEditors.get(node.id)?.dispose();
      this.embeddedEditors.delete(node.id);
      const createdEditor = element.ownerDocument.createElement("div");
      editor = createdEditor;
      createdEditor.className = "zeta-document-rich-text-input";
      createdEditor.dataset.blockId = node.id;
      createdEditor.contentEditable = this.isReadOnly() ? "false" : "true";
      createdEditor.setAttribute("aria-readonly", String(this.isReadOnly()));
      createdEditor.setAttribute("role", "textbox");
      createdEditor.addEventListener("beforeinput", event => this.handleRichTextBeforeInput(event as InputEvent, model, createdEditor));
      createdEditor.addEventListener("paste", event => this.handleRichTextPaste(event, model, createdEditor));
      createdEditor.addEventListener("copy", event => this.handleRichTextClipboard(event, model, false));
      createdEditor.addEventListener("cut", event => this.handleRichTextClipboard(event, model, true));
      createdEditor.addEventListener("keydown", event => this.handleRichTextKeydown(event, node, model, createdEditor));
      createdEditor.addEventListener("input", () => this.handleRichTextInput(createdEditor, model));
      createdEditor.addEventListener("focus", () => this.syncRichTextSelection(createdEditor, model));
      createdEditor.addEventListener("keyup", () => this.syncRichTextSelection(createdEditor, model));
      createdEditor.addEventListener("mouseup", () => this.syncRichTextSelection(createdEditor, model, true));
      createdEditor.addEventListener("compositionstart", () => this.beginComposition(model, node.id, createdEditor, readDocumentTextSelection(this.requireContainer(), true)?.selection ?? (model.selection?.kind === "text" ? model.selection : undefined)));
      createdEditor.addEventListener("compositionend", event => this.endComposition(event, createdEditor));
      createdEditor.addEventListener("compositioncancel", () => this.cancelComposition(createdEditor));
      element.replaceChildren(createdEditor);
    }
    editor.dataset.blockId = node.id;
    this.renderInlineContent(editor, node, model, decorations);
  }

  private renderInlineContent(editor: HTMLDivElement, node: DocumentNode, model: DocumentModel, decorations: readonly ViewDecoration[]): void {
    const fragment = editor.ownerDocument.createDocumentFragment();
    for (const child of node.content) {
      if (child.text !== undefined) {
        const linkMark = child.marks.find(mark => mark.type === "link");
        const start = documentPointToPosition(model.document, model.schema, { nodeId: child.id, offset: 0 });
        const end = documentPointToPosition(model.document, model.schema, { nodeId: child.id, offset: child.text.length });
        const localDecorations = decorations.filter(decoration => decoration.to > start && decoration.from < end);
        const boundaries = new Set([0, child.text.length]);
        for (const decoration of localDecorations) {
          boundaries.add(Math.max(0, Math.min(child.text.length, decoration.from - start)));
          boundaries.add(Math.max(0, Math.min(child.text.length, decoration.to - start)));
        }
        const offsets = [...boundaries].sort((left, right) => left - right);
        for (let index = 0; index < Math.max(1, offsets.length - 1); index += 1) {
          const from = offsets[index] ?? 0;
          const to = offsets[index + 1] ?? child.text.length;
          if (to < from) continue;
          const activeDecorations = localDecorations.filter(decoration => decoration.from < start + to && decoration.to > start + from).map(decoration => decoration.decoration);
          const run = editor.ownerDocument.createElement(linkMark ? "a" : "span");
          run.className = "zeta-document-inline-run";
          run.dataset.textNodeId = child.id;
          for (const mark of child.marks) run.classList.add(`zeta-document-mark-${mark.type}`);
          if (linkMark) {
            run.setAttribute("href", typeof linkMark.attrs.href === "string" ? linkMark.attrs.href : "");
            run.addEventListener("click", event => event.preventDefault());
          }
          run.textContent = child.text.slice(from, to);
          applyViewDecorations(run, activeDecorations);
          fragment.append(run);
        }
        continue;
      }
      if (child.type === "hardBreak") {
        fragment.append(editor.ownerDocument.createElement("br"));
        continue;
      }
      if (child.type === "image") {
        const image = editor.ownerDocument.createElement("img");
        image.className = "zeta-document-inline-image";
        image.dataset.inlineNodeId = child.id;
        image.draggable = false;
        image.alt = typeof child.attrs.alt === "string" ? child.attrs.alt : "";
        image.src = typeof child.attrs.src === "string" ? child.attrs.src : "";
        image.addEventListener("click", event => {
          event.preventDefault();
          this.selectInlineNode(model, node.id, child.id, editor);
        });
        fragment.append(image);
        continue;
      }
      const inlineFactory = this.options.inlineNodeViews?.[child.type];
      const inlineElement = inlineFactory
        ? inlineFactory({ node: child, model, ownerDocument: editor.ownerDocument, select: () => this.selectInlineNode(model, node.id, child.id, editor) })
        : createFallbackInlineNode(editor.ownerDocument, child);
      if (!inlineElement || inlineElement.nodeType !== 1) throw new TypeError(`Gamma inline node view '${child.type}' must return an HTMLElement`);
      inlineElement.dataset.inlineNodeId = child.id;
      inlineElement.dataset.inlineNodeType = child.type;
      inlineElement.addEventListener("click", event => {
        event.preventDefault();
        this.selectInlineNode(model, node.id, child.id, editor);
      });
      fragment.append(inlineElement);
    }
    editor.replaceChildren(fragment);
  }

  private handleRichTextInput(editor: HTMLDivElement, model: DocumentModel): void {
    if (this.modelSlot.value !== model) return;
    if (this.isReadOnly() || !this.requireContainer().contains(editor) || this.composition?.element === editor) return;
    const blockId = editor.dataset.blockId;
    if (!blockId) return;
    const node = findNode(model.document, blockId);
    if (!node) return;
    const textNodes = node.content.filter(child => child.text !== undefined);
    const runs = Array.from(editor.querySelectorAll<HTMLElement>("[data-text-node-id]"));
    const runsById = new Map<string, HTMLElement[]>();
    for (const run of runs) {
      const nodeId = run.dataset.textNodeId;
      if (!nodeId) continue;
      const nodeRuns = runsById.get(nodeId) ?? [];
      nodeRuns.push(run);
      runsById.set(nodeId, nodeRuns);
    }
    if (textNodes.every(textNode => runsById.has(textNode.id)) && runsById.size === textNodes.length) {
      let transaction = new DocumentTransaction();
      for (const textNode of textNodes) {
        const nextText = (runsById.get(textNode.id) ?? []).map(run => run.textContent ?? "").join("");
        if (nextText !== textNode.text) transaction = transaction.replaceText(textNode.id, 0, textNode.text!.length, nextText);
      }
      if (transaction.steps.length > 0) model.dispatch(transaction.withHistoryGroup("typing"));
      return;
    }
    const text = editor.textContent ?? "";
    let transaction = new DocumentTransaction();
    for (const textNode of textNodes) transaction = transaction.deleteNode(textNode.id);
    if (text.length > 0) transaction = transaction.insertNode(node.id, 0, model.schema.createText(text, textNodes[0] ? { id: textNodes[0].id } : {}));
    if (transaction.steps.length > 0) model.dispatch(transaction.withHistoryGroup("typing"));
  }

  private beginComposition(model: DocumentModel, blockId: string, element: HTMLTextAreaElement | HTMLDivElement, selection: TextSelection | undefined): void {
    if (this.isReadOnly() || this.modelSlot.value !== model || !selection || selection.kind !== "text") return;
    if (findTextBlockId(model.document, selection.anchor.nodeId) !== blockId) return;
    this.composition = { model, blockId, element, selection, baseText: readCompositionText(element), version: model.version };
    if (model.selection?.kind !== "text" || model.selection.anchor.nodeId !== selection.anchor.nodeId || model.selection.anchor.offset !== selection.anchor.offset || model.selection.head.nodeId !== selection.head.nodeId || model.selection.head.offset !== selection.head.offset) model.setSelection(selection);
  }

  private endComposition(event: CompositionEvent, element: HTMLTextAreaElement | HTMLDivElement): void {
    const composition = this.composition;
    if (!composition || composition.element !== element) return;
    this.composition = undefined;
    const model = composition.model;
    if (this.isReadOnly() || this.modelSlot.value !== model || model.version !== composition.version) {
      if (this.modelSlot.value === model) this.render();
      return;
    }
    const currentText = readCompositionText(element);
    let selection = composition.selection;
    let replacement = event.data ?? "";
    if (replacement.length === 0 && currentText !== composition.baseText) {
      const diff = findCompositionDiff(composition.baseText, currentText);
      const textNode = findSingleTextNodeInBlock(model.document, composition.blockId);
      if (diff && textNode) {
        selection = textSelection({ nodeId: textNode.id, offset: diff.from }, { nodeId: textNode.id, offset: diff.to });
        replacement = diff.text;
      }
    }
    if (replacement.length === 0 && currentText === composition.baseText) return;
    const command = createReplaceTextCommand(model.schema, model.document, composition.blockId, selection, replacement, documentInsertionMarks(model, selection));
    if (!command) {
      this.render();
      return;
    }
    this.dispatchCommand(model, { ...command, transaction: command.transaction.withMeta("inputType", "insertCompositionText") }, "composition");
  }

  private cancelComposition(element: HTMLTextAreaElement | HTMLDivElement): void {
    if (this.composition?.element !== element) return;
    const model = this.composition.model;
    this.composition = undefined;
    if (this.modelSlot.value === model) this.render();
  }

  private handleTextPaste(event: ClipboardEvent, model: DocumentModel, blockId: string, textarea: HTMLTextAreaElement): void {
    if (this.isReadOnly()) {
      event.preventDefault();
      return;
    }
    if (this.modelSlot.value !== model) return;
    const image = findImageClipboardFile(event.clipboardData);
    if (image) {
      event.preventDefault();
      void this.insertPastedImage(model, blockId, image, createTextareaTextSelection(model.document, blockId, textarea));
      return;
    }
    const selection = model.selection?.kind === "all" ? model.selection : createTextareaTextSelection(model.document, blockId, textarea);
    const encodedFragment = event.clipboardData?.getData(DOCUMENT_FRAGMENT_CLIPBOARD_MIME);
    if (encodedFragment && selection) {
      let fragment;
      try {
        fragment = deserializeDocumentFragment(encodedFragment, model.schema);
      } catch {
        fragment = undefined;
      }
      const command = fragment ? createInsertFragmentCommand(model.schema, model.document, blockId, selection, fragment) : undefined;
      if (command) {
        event.preventDefault();
        this.dispatchCommand(model, command);
        return;
      }
    }
    if (model.selection?.kind !== "all") return;
    const text = event.clipboardData?.getData("text/plain") ?? "";
    const command = createPasteTextCommand(model.schema, model.document, blockId, model.selection, text);
    if (!command) return;
    event.preventDefault();
    this.dispatchCommand(model, command);
  }

  private handleRichTextPaste(event: ClipboardEvent, model: DocumentModel, editor: HTMLDivElement): void {
    if (this.isReadOnly()) {
      event.preventDefault();
      return;
    }
    if (this.modelSlot.value !== model) return;
    const image = findImageClipboardFile(event.clipboardData);
    const blockId = editor.dataset.blockId;
    if (image && blockId) {
      event.preventDefault();
      void this.insertPastedImage(model, blockId, image, readDocumentTextSelection(this.requireContainer(), true)?.selection);
      return;
    }
    if (!blockId) return;
    const encodedFragment = event.clipboardData?.getData(DOCUMENT_FRAGMENT_CLIPBOARD_MIME);
    if (!encodedFragment) return;
    let fragment;
    try {
      fragment = deserializeDocumentFragment(encodedFragment, model.schema);
    } catch {
      return;
    }
    const selection = model.selection?.kind === "all" ? model.selection : readDocumentTextSelection(this.requireContainer(), true)?.selection;
    const command = selection ? createInsertFragmentCommand(model.schema, model.document, blockId, selection, fragment) : undefined;
    if (!command) return;
    event.preventDefault();
    this.dispatchCommand(model, command);
  }

  private handleRichTextClipboard(event: ClipboardEvent, model: DocumentModel, cut: boolean): void {
    if (this.modelSlot.value !== model) return;
    if (cut && this.isReadOnly()) {
      event.preventDefault();
      return;
    }
    if (model.selection?.kind !== "all") {
      const domSelection = readDocumentTextSelection(this.requireContainer(), true);
      if (domSelection) model.setSelection(domSelection.selection);
    }
    const selection = model.selection;
    if (!selection || (selection.kind === "text" && isCollapsedTextSelection(selection)) || selection.kind === "node") return;
    const text = documentSelectionToText(model.document, selection);
    if (text === undefined) return;
    const fragment = extractDocumentFragment(model.schema, model.document, selection);
    const encodedFragment = fragment ? serializeDocumentFragment(fragment, model.schema) : undefined;
    let command: DocumentCommand | undefined;
    if (cut) {
      const blockId = selection.kind === "all" ? findFirstEditableBlock(model.document)?.id : findTextBlockId(model.document, selection.anchor.nodeId);
      if (!blockId) return;
      command = createDeleteInlineSelectionCommand(model.schema, model.document, blockId, selection) ?? createReplaceTextCommand(model.schema, model.document, blockId, selection, "");
      if (!command) return;
    }
    event.preventDefault();
    event.clipboardData?.setData("text/plain", text);
    if (encodedFragment) event.clipboardData?.setData(DOCUMENT_FRAGMENT_CLIPBOARD_MIME, encodedFragment);
    if (command) this.dispatchCommand(model, command);
  }

  private handleTextClipboard(event: ClipboardEvent, model: DocumentModel, blockId: string, textarea: HTMLTextAreaElement, cut: boolean): void {
    if (this.modelSlot.value !== model) return;
    if (cut && this.isReadOnly()) {
      event.preventDefault();
      return;
    }
    if (model.selection?.kind !== "all") {
      const selection = createTextareaTextSelection(model.document, blockId, textarea);
      if (selection) model.setSelection(selection);
    }
    const selection = model.selection;
    if (!selection || (selection.kind === "text" && isCollapsedTextSelection(selection)) || selection.kind === "node") return;
    const text = documentSelectionToText(model.document, selection);
    if (text === undefined) return;
    const fragment = extractDocumentFragment(model.schema, model.document, selection);
    const encodedFragment = fragment ? serializeDocumentFragment(fragment, model.schema) : undefined;
    let command: DocumentCommand | undefined;
    if (cut) {
      command = createDeleteInlineSelectionCommand(model.schema, model.document, blockId, selection) ?? createReplaceTextCommand(model.schema, model.document, blockId, selection, "");
      if (!command) return;
    }
    event.preventDefault();
    event.clipboardData?.setData("text/plain", text);
    if (encodedFragment) event.clipboardData?.setData(DOCUMENT_FRAGMENT_CLIPBOARD_MIME, encodedFragment);
    if (command) this.dispatchCommand(model, command);
  }

  private async insertPastedImage(model: DocumentModel, blockId: string, image: File, selection?: TextSelection): Promise<void> {
    if (this.isReadOnly()) return;
    const ownerDocument = this.container?.ownerDocument;
    if (!ownerDocument) return;
    let src: string;
    try {
      src = await readBlobAsDataUrl(image, ownerDocument);
    } catch {
      return;
    }
    if (this.modelSlot.value !== model) return;
    if (selection) model.setSelection(selection);
    const command = selection
      ? createInsertImageAtSelectionCommand(model.schema, model.document, blockId, selection, src, image.name) ?? createInsertImageCommand(model.schema, model.document, blockId, src, image.name)
      : createInsertImageCommand(model.schema, model.document, blockId, src, image.name);
    if (command) this.dispatchCommand(model, command);
  }

  private handleRichTextBeforeInput(event: InputEvent, model: DocumentModel, editor: HTMLDivElement): void {
    if (this.isReadOnly() || this.modelSlot.value !== model || event.isComposing || event.inputType === "insertCompositionText" || event.inputType === "deleteCompositionText") return;
    const blockId = editor.dataset.blockId;
    if (!blockId) return;
    if (model.selection?.kind === "all") {
      let command: DocumentCommand | undefined;
      if (event.inputType === "insertText" || event.inputType === "insertFromPaste" || event.inputType === "insertFromDrop") {
        const text = event.data ?? event.dataTransfer?.getData("text/plain") ?? "";
        const marks = documentInsertionMarks(model, model.selection);
        command = event.inputType === "insertFromPaste" ? createPasteTextCommand(model.schema, model.document, blockId, model.selection, text, marks) : createReplaceTextCommand(model.schema, model.document, blockId, model.selection, text, marks);
      } else if (event.inputType === "deleteContentBackward" || event.inputType === "deleteContentForward") {
        command = createDeleteInlineSelectionCommand(model.schema, model.document, blockId, model.selection);
      }
      if (command) {
        event.preventDefault();
        this.dispatchCommand(model, command, event.inputType === "insertText" ? "typing" : undefined);
      }
      return;
    }
    if (model.selection?.kind === "node" && isInlineNodeInBlock(model.document, blockId, model.selection.nodeId) && (event.inputType === "deleteContentBackward" || event.inputType === "deleteContentForward")) {
      const command = createDeleteNodeSelectionCommand(model.document, model.selection);
      if (command) {
        event.preventDefault();
        this.dispatchCommand(model, command);
      }
      return;
    }
    if (event.inputType === "insertFromPaste") {
      const image = findImageClipboardFile(event.dataTransfer);
      if (image) {
        event.preventDefault();
        void this.insertPastedImage(model, blockId, image, readDocumentTextSelection(this.requireContainer(), true)?.selection);
        return;
      }
    }
    const inlineSelection = readDocumentTextSelection(this.requireContainer(), true);
    if (!inlineSelection) return;
    const selection = inlineSelection.selection;
    let command: DocumentCommand | undefined;
    if (event.inputType === "insertText" || event.inputType === "insertCompositionText" || event.inputType === "insertFromPaste" || event.inputType === "insertFromDrop") {
      const text = event.data ?? event.dataTransfer?.getData("text/plain") ?? "";
      const marks = documentInsertionMarks(model, selection);
      if (text.length > 0) command = event.inputType === "insertFromPaste" && text.includes("\n")
        ? createPasteTextCommand(model.schema, model.document, blockId, selection, text, marks)
        : createReplaceTextCommand(model.schema, model.document, blockId, selection, text, marks);
    } else if (event.inputType === "deleteContentBackward") {
      command = createDeleteBoundaryCommand(model, blockId, selection, "backward");
    } else if (event.inputType === "deleteContentForward") {
      command = createDeleteBoundaryCommand(model, blockId, selection, "forward");
    } else if (event.inputType === "insertLineBreak") {
      command = createInsertHardBreakCommand(model.schema, model.document, blockId, selection);
    } else if (event.inputType === "insertParagraph" && isCollapsedTextSelection(selection)) {
      command = createParagraphSplitCommand(model.schema, model.document, blockId, selection.anchor.nodeId, selection.anchor.offset);
    }
    if (!command) return;
    event.preventDefault();
    const historyGroup = event.inputType === "insertText" ? "typing" : event.inputType === "deleteContentBackward" ? "delete-backward" : event.inputType === "deleteContentForward" ? "delete-forward" : undefined;
    this.dispatchCommand(model, command, historyGroup);
  }

  private syncRichTextSelection(editor: HTMLDivElement, model: DocumentModel, force = false): void {
    if (this.modelSlot.value !== model) return;
    const inlineSelection = readDocumentTextSelection(this.requireContainer(), true);
    this.activeBlockId = inlineSelection?.blockId ?? editor.dataset.blockId;
    if (inlineSelection && !(model.selection?.kind === "all" && isCollapsedTextSelection(inlineSelection.selection) && !force)) model.setSelection(inlineSelection.selection);
    this.updateToolbar();
    this.updateInlineNodeSelection();
  }

  private syncDocumentSelection(): void {
    const model = this.modelSlot.value;
    const container = this.container;
    if (!model || !container) return;
    const inlineSelection = readDocumentTextSelection(container, true);
    if (!inlineSelection) return;
    this.activeBlockId = inlineSelection.blockId;
    if (!(model.selection?.kind === "all" && isCollapsedTextSelection(inlineSelection.selection))) model.setSelection(inlineSelection.selection);
    this.updateToolbar();
    this.updateInlineNodeSelection();
  }

  private handleRichTextKeydown(event: KeyboardEvent, node: DocumentNode, model: DocumentModel, editor: HTMLDivElement): void {
    if (this.isReadOnly() || event.isComposing) return;
    if (this.handleHistoryShortcut(event, model)) return;
    if ((event.metaKey || event.ctrlKey) && !event.altKey && !event.shiftKey && event.key.toLowerCase() === "a") {
      event.preventDefault();
      model.setSelection(allSelection());
      this.activeBlockId = node.id;
      this.updateToolbar();
      this.updateInlineNodeSelection();
      return;
    }
    if (model.selection?.kind === "all" && !event.metaKey && !event.ctrlKey && !event.altKey && (event.key === "Backspace" || event.key === "Delete")) {
      const command = createDeleteInlineSelectionCommand(model.schema, model.document, node.id, model.selection);
      if (command) {
        event.preventDefault();
        this.dispatchCommand(model, command, event.key === "Backspace" ? "delete-backward" : "delete-forward");
      }
      return;
    }
    const selectedNode = model.selection?.kind === "node" ? model.selection : undefined;
    if (selectedNode && node.content.some(child => child.id === selectedNode.nodeId)) {
      if (!event.metaKey && !event.ctrlKey && !event.altKey && (event.key === "Backspace" || event.key === "Delete")) {
        const command = createDeleteNodeSelectionCommand(model.document, selectedNode);
        if (command) {
          event.preventDefault();
          this.dispatchCommand(model, command);
        }
      }
      return;
    }
    const inlineSelection = readDocumentTextSelection(this.requireContainer(), true);
    if (!inlineSelection) return;
    const selection = inlineSelection.selection;
    const textNode = findNode(model.document, inlineSelection.nodeId);
    if (!textNode || textNode.text === undefined) return;
    const collapsed = isCollapsedTextSelection(selection);
    let command: DocumentCommand | undefined;
    if (event.key === "Tab" && !event.metaKey && !event.ctrlKey && !event.altKey && collapsed) {
      if (this.handleTableCellTab(event, model, node.id)) return;
      command = createListItemIndentationForBlock(model.schema, model.document, node.id, event.shiftKey ? "out" : "in");
    } else if ((event.metaKey || event.ctrlKey) && !event.altKey && !event.shiftKey && (event.key === "b" || event.key === "B" || event.key === "i" || event.key === "I")) {
      command = createToggleMarkCommand(model.schema, model.document, node.id, inlineSelection.nodeId, selection, event.key.toLowerCase() === "b" ? "strong" : "em", {}, model.storedMarks);
    } else if ((event.metaKey || event.ctrlKey) && !event.altKey && !event.shiftKey && (event.key === "k" || event.key === "K") && !collapsed) {
      const href = editor.ownerDocument.defaultView?.prompt("Link URL", "https://");
      if (href) command = createSetLinkMarkCommand(model.schema, model.document, node.id, inlineSelection.nodeId, selection, href, model.storedMarks);
    } else if (!event.metaKey && !event.ctrlKey && !event.altKey && event.key === "Enter" && event.shiftKey) {
      command = createInsertHardBreakCommand(model.schema, model.document, node.id, selection);
    } else if (!event.metaKey && !event.ctrlKey && event.key === "Enter" && !event.shiftKey && collapsed) {
      command = createParagraphSplitCommand(model.schema, model.document, node.id, inlineSelection.nodeId, selection.anchor.offset);
    } else if (!event.metaKey && !event.ctrlKey && !event.altKey && (event.key === "Backspace" || event.key === "Delete") && !collapsed) {
      command = createDeleteInlineSelectionCommand(model.schema, model.document, node.id, selection) ?? createReplaceTextCommand(model.schema, model.document, node.id, selection, "");
    } else if (!event.metaKey && !event.ctrlKey && event.key === "Backspace" && collapsed && selection.anchor.offset === 0) {
      command = createInlineBoundaryCommand(model.document, node.id, inlineSelection.nodeId, "backward");
    } else if (!event.metaKey && !event.ctrlKey && event.key === "Delete" && collapsed && selection.anchor.offset === textNode.text.length) {
      command = createInlineBoundaryCommand(model.document, node.id, inlineSelection.nodeId, "forward");
    }
    if (!command) return;
    event.preventDefault();
    const historyGroup = event.key === "Backspace" ? "delete-backward" : event.key === "Delete" ? "delete-forward" : undefined;
    this.dispatchCommand(model, command, historyGroup);
  }

  private handleTextKeydown(event: KeyboardEvent, node: DocumentNode, model: DocumentModel, textarea: HTMLTextAreaElement): void {
    const currentModel = this.modelSlot.value;
    if (this.isReadOnly() || currentModel !== model || event.isComposing) return;
    if (this.handleHistoryShortcut(event, model)) return;
    if ((event.metaKey || event.ctrlKey) && !event.altKey && !event.shiftKey && event.key.toLowerCase() === "a") {
      event.preventDefault();
      model.setSelection(allSelection());
      textarea.select();
      this.activeBlockId = node.id;
      this.updateToolbar();
      return;
    }
    if (model.selection?.kind === "all" && !event.metaKey && !event.ctrlKey && !event.altKey && (event.key === "Backspace" || event.key === "Delete")) {
      const command = createDeleteInlineSelectionCommand(model.schema, model.document, node.id, model.selection);
      if (command) {
        event.preventDefault();
        this.dispatchCommand(model, command, event.key === "Backspace" ? "delete-backward" : "delete-forward");
      }
      return;
    }
    const currentNode = findNode(model.document, node.id);
    if (!currentNode || (currentNode.type !== "paragraph" && currentNode.type !== "heading")) return;
    const textNode = currentNode.content.length === 1 && currentNode.content[0]?.text !== undefined ? currentNode.content[0] : undefined;
    const start = textarea.selectionStart ?? 0;
    const end = textarea.selectionEnd ?? start;
    let command: DocumentCommand | undefined;
    if (event.key === "Tab" && !event.metaKey && !event.ctrlKey && !event.altKey) {
      if (this.handleTableCellTab(event, model, node.id)) return;
      command = createListItemIndentationForBlock(model.schema, model.document, node.id, event.shiftKey ? "out" : "in");
    } else if ((event.metaKey || event.ctrlKey) && event.key === "Enter") {
      command = createInsertParagraphAfterCommand(model.schema, model.document, node.id);
    } else if ((event.metaKey || event.ctrlKey) && !event.altKey && !event.shiftKey && textNode && (event.key === "b" || event.key === "B" || event.key === "i" || event.key === "I")) {
      command = createToggleMarkCommand(model.schema, model.document, node.id, textNode.id, textSelection({ nodeId: textNode.id, offset: start }, { nodeId: textNode.id, offset: end }), event.key.toLowerCase() === "b" ? "strong" : "em", {}, model.storedMarks);
    } else if (event.altKey && !event.metaKey && !event.ctrlKey && event.key === "ArrowUp") {
      command = createMoveBlockCommand(model.document, node.id, "up");
    } else if (event.altKey && !event.metaKey && !event.ctrlKey && event.key === "ArrowDown") {
      command = createMoveBlockCommand(model.document, node.id, "down");
    } else if (event.key === "Enter" && event.shiftKey && !event.metaKey && !event.ctrlKey && !event.altKey) {
      command = createInsertHardBreakCommand(model.schema, model.document, node.id, createTextareaTextSelection(model.document, node.id, textarea));
    } else if (event.key === "Enter" && !event.shiftKey && start === end) {
      command = textNode
        ? createParagraphSplitCommand(model.schema, model.document, node.id, textNode.id, start)
        : createParagraphSplitCommand(model.schema, model.document, node.id, "", 0) ?? createInsertParagraphAfterCommand(model.schema, model.document, node.id);
    } else if (event.key === "Backspace" && start === 0 && end === 0) {
      command = createJoinAdjacentBlockCommand(model.document, node.id, textNode?.id ?? "", "backward");
    } else if (event.key === "Delete" && textNode && start === textNode.text!.length && end === start) {
      command = createJoinAdjacentBlockCommand(model.document, node.id, textNode.id, "forward");
    }
    if (!command) return;
    event.preventDefault();
    const historyGroup = event.key === "Backspace" ? "delete-backward" : event.key === "Delete" ? "delete-forward" : undefined;
    this.dispatchCommand(model, command, historyGroup);
  }

  private handleHistoryShortcut(event: KeyboardEvent, model: DocumentModel): boolean {
    const action = historyShortcut(event);
    if (!action) return false;
    event.preventDefault();
    const change = action === "undo" ? model.undo() : model.redo();
    if (change) this.restoreModelFocus(model, change.selectionAfter);
    return true;
  }

  private restoreModelFocus(model: DocumentModel, selection: DocumentSelection | undefined): void {
    let blockId: string | undefined;
    if (selection?.kind === "text") blockId = findTextBlockId(model.document, selection.anchor.nodeId);
    if (selection?.kind === "node") blockId = findTextBlockContainingNode(model.document, selection.nodeId);
    if (!blockId && this.activeBlockId) {
      const active = findNode(model.document, this.activeBlockId);
      if (active) blockId = findFirstEditableBlock(active)?.id;
    }
    blockId ??= findFirstEditableBlock(model.document)?.id;
    if (!blockId) return;
    this.activeBlockId = blockId;
    const editor = findBlockEditor(this.requireContainer(), blockId);
    if (!editor) {
      this.updateToolbar();
      this.updateInlineNodeSelection();
      return;
    }
    editor.focus();
    if (selection?.kind === "text" && editor.tagName === "TEXTAREA") {
      const textarea = editor as HTMLTextAreaElement;
      const block = findNode(model.document, blockId);
      const textNode = block?.content.length === 1 && block.content[0]?.text !== undefined ? block.content[0] : undefined;
      if (textNode) {
        const anchor = Math.min(textNode.text!.length, selection.anchor.offset);
        const head = Math.min(textNode.text!.length, selection.head.offset);
        textarea.setSelectionRange(Math.min(anchor, head), Math.max(anchor, head), anchor <= head ? "forward" : "backward");
      }
    } else if (selection?.kind === "text" && editor.classList.contains("zeta-document-rich-text-input")) {
      setInlineTextSelection(this.requireContainer(), selection);
    }
    this.updateToolbar();
    this.updateInlineNodeSelection();
  }

  private selectInlineNode(model: DocumentModel, blockId: string, nodeId: string, editor: HTMLDivElement): void {
    if (this.modelSlot.value !== model) return;
    editor.focus();
    model.setSelection(nodeSelection(nodeId));
    this.activeBlockId = blockId;
    this.updateInlineNodeSelection();
    this.updateToolbar();
  }

  private updateInlineNodeSelection(): void {
    const container = this.container;
    const model = this.modelSlot.value;
    if (!container || !model) return;
    const selectedNodeId = model.selection?.kind === "node" ? model.selection.nodeId : undefined;
    for (const element of container.querySelectorAll<HTMLElement>("[data-inline-node-id]")) {
      const selected = element.dataset.inlineNodeId === selectedNodeId;
      element.classList.toggle("zeta-document-inline-node-selected", selected);
      element.setAttribute("aria-selected", String(selected));
    }
  }

  private handleTableCellTab(event: KeyboardEvent, model: DocumentModel, blockId: string): boolean {
    const context = findTableCellContext(model.document, blockId);
    if (!context) return false;
    event.preventDefault();
    const direction = event.shiftKey ? "backward" : "forward";
    const targetCellId = findAdjacentTableCell(model.document, context.cell.id, direction);
    if (targetCellId) {
      const targetCell = findNode(model.document, targetCellId);
      const targetBlock = targetCell ? findFirstEditableBlock(targetCell) : undefined;
      if (targetBlock) this.focusBlockAtBoundary(model, targetBlock.id, direction);
      return true;
    }
    if (direction === "forward") {
      const command = createInsertTableRowCommand(model.schema, model.document, context.table.id, context.table.content.length);
      if (command) this.dispatchCommand(model, command);
    }
    return true;
  }

  private focusBlockAtBoundary(model: DocumentModel, blockId: string, direction: "backward" | "forward"): void {
    const editor = findBlockEditor(this.requireContainer(), blockId);
    if (!editor) return;
    this.activeBlockId = blockId;
    editor.focus();
    const node = findNode(model.document, blockId);
    const textNode = node ? findFirstTextNode(node) : undefined;
    if (editor.tagName === "TEXTAREA") {
      const textarea = editor as HTMLTextAreaElement;
      const offset = direction === "backward" ? textarea.value.length : 0;
      textarea.setSelectionRange(offset, offset);
    } else if (textNode) {
      const offset = direction === "backward" ? textNode.text!.length : 0;
      const selection = textSelection({ nodeId: textNode.id, offset });
      model.setSelection(selection);
      setInlineTextSelection(editor as HTMLDivElement, selection);
    }
    this.updateToolbar();
  }

  private dispatchCommand(model: DocumentModel, command: DocumentCommand, historyGroup?: string): void {
    if (this.isReadOnly() || this.modelSlot.value !== model) return;
    this.activeBlockId = command.focus.blockId;
    model.dispatch(historyGroup ? command.transaction.withHistoryGroup(historyGroup) : command.transaction);
    const editor = findBlockEditor(this.requireContainer(), command.focus.blockId);
    if (!editor) return;
    editor.focus();
    if (editor.tagName === "TEXTAREA") {
      const textarea = editor as HTMLTextAreaElement;
      const offset = command.focus.point?.offset ?? 0;
      textarea.setSelectionRange(Math.min(offset, textarea.value.length), Math.min(offset, textarea.value.length));
    } else if (editor.classList.contains("zeta-document-rich-text-input") && model.selection?.kind === "text") {
      setInlineTextSelection(this.requireContainer(), model.selection);
    }
    this.updateToolbar();
  }

  private handleToolbarAction(action: string): void {
    if (this.isReadOnly()) return;
    const model = this.modelSlot.value;
    if (!model) return;
    const blockId = this.activeBlockId ?? findTextBlockId(model.document, model.selection?.kind === "text" ? model.selection.anchor.nodeId : undefined);
    if (!blockId) return;
    const selection = this.readActiveTextSelection(model, blockId);
    const selectionBlockId = selection ? findTextBlockId(model.document, selection.anchor.nodeId) : undefined;
    let command: DocumentCommand | undefined;
    switch (action) {
      case "paragraph":
      case "heading":
        command = createSetBlockTypeCommand(model.document, blockId, action);
        break;
      case "blockquote":
        command = createToggleBlockquoteCommand(this.schema, model.document, blockId);
        break;
      case "bulletList":
      case "orderedList":
        command = createToggleListCommand(this.schema, model.document, blockId, action);
        break;
      case "horizontalRule":
        command = createInsertHorizontalRuleCommand(this.schema, model.document, blockId);
        break;
      case "link": {
        if (!selection || !selectionBlockId) break;
        const href = this.requireContainer().ownerDocument.defaultView?.prompt("Link URL", "https://");
        if (href) command = createSetLinkMarkCommand(this.schema, model.document, selectionBlockId, selection.anchor.nodeId, selection, href, model.storedMarks);
        break;
      }
      case "unlink":
        if (selection && selectionBlockId) command = createRemoveMarkCommand(this.schema, model.document, selectionBlockId, selection.anchor.nodeId, selection, "link", model.storedMarks);
        break;
      case "table":
        command = createInsertTableCommand(this.schema, model.document, blockId);
        break;
      case "insertTableRow": {
        const context = findTableCellContext(model.document, blockId);
        if (context) command = createInsertTableRowCommand(this.schema, model.document, context.table.id, context.rowIndex + 1);
        break;
      }
      case "insertTableColumn": {
        const context = findTableCellContext(model.document, blockId);
        if (context) command = createInsertTableColumnCommand(this.schema, model.document, context.table.id, context.columnIndex + 1);
        break;
      }
      case "deleteTableRow": {
        const context = findTableCellContext(model.document, blockId);
        if (context) command = createDeleteTableRowCommand(model.document, context.table.id, context.row.id);
        break;
      }
      case "deleteTableColumn": {
        const context = findTableCellContext(model.document, blockId);
        if (context) command = createDeleteTableColumnCommand(model.document, context.table.id, context.columnIndex);
        break;
      }
    }
    if (!command) {
      const toolbarAction = this.options.toolbarActions?.find(candidate => candidate.id === action);
      if (toolbarAction) command = toolbarAction.run({ model, blockId, selection, ownerDocument: this.requireContainer().ownerDocument });
    }
    if (!command) return;
    this.dispatchCommand(model, command);
  }

  private readActiveTextSelection(model: DocumentModel, blockId: string): TextSelection | undefined {
    const editor = findBlockEditor(this.requireContainer(), blockId);
    if (editor?.tagName === "TEXTAREA") return createTextareaTextSelection(model.document, blockId, editor as HTMLTextAreaElement);
    if (editor?.classList.contains("zeta-document-rich-text-input")) {
      const inlineSelection = readDocumentTextSelection(this.requireContainer(), true);
      if (inlineSelection?.blockId === blockId) return inlineSelection.selection;
    }
    const selection = model.selection;
    return selection?.kind === "text" && findTextBlockId(model.document, selection.anchor.nodeId) === blockId ? selection : undefined;
  }

  private updateToolbar(): void {
    const toolbar = this.toolbar;
    const model = this.modelSlot.value;
    if (!toolbar || !model) return;
    const blockId = this.activeBlockId ?? findTextBlockId(model.document, model.selection?.kind === "text" ? model.selection.anchor.nodeId : undefined);
    const block = blockId ? findNode(model.document, blockId) : undefined;
    const list = block ? findListForBlock(model.document, block.id) : undefined;
    const selection = model.selection?.kind === "text" ? model.selection : undefined;
    const selectionBlock = selection ? findNode(model.document, findTextBlockId(model.document, selection.anchor.nodeId) ?? "") : undefined;
    const readOnly = this.isReadOnly();
    for (const button of toolbar.querySelectorAll<HTMLButtonElement>("button[data-block-type]")) {
      const type = button.dataset.blockType;
      let checked = false;
      if (type === "paragraph" || type === "heading") checked = block?.type === type;
      else if (type === "blockquote") checked = block !== undefined && findBlockquoteForBlock(model.document, block.id) !== undefined;
      else if (type === "link") checked = selection !== undefined && (selectionBlock?.type === "paragraph" || selectionBlock?.type === "heading") && isTextSelectionMarked(selectionBlock, selection, "link", model.storedMarks);
      else checked = list?.type === type;
      button.classList.toggle("checked", checked === true);
      button.setAttribute("aria-pressed", String(checked === true));
      button.disabled = readOnly;
      button.setAttribute("aria-disabled", String(readOnly));
    }
  }

  private renderChildren(element: HTMLElement, node: DocumentNode, model: DocumentModel, previousElements: Map<string, HTMLElement>, activeNodeIds: Set<string>, decorations: readonly ViewDecoration[]): void {
    const fragment = element.ownerDocument.createDocumentFragment();
    for (const child of node.content) fragment.append(this.renderNode(child, model, previousElements, activeNodeIds, decorations));
    element.replaceChildren(fragment);
  }

  private reuseElement(node: DocumentNode, previousElements: Map<string, HTMLElement>, document: Document, tagName: string): HTMLElement {
    const previous = previousElements.get(node.id);
    if (previous?.tagName.toLowerCase() === tagName) return previous;
    return document.createElement(tagName);
  }

  private requireContainer(): HTMLDivElement {
    const container = this.container;
    assertDefined(container, new ReferenceError("DocumentEditorPane has not been created"));
    return container;
  }

  private requireModel(): DocumentModel {
    const model = this.modelSlot.value;
    assertDefined(model, new ReferenceError("DocumentEditorPane has no active model"));
    return model;
  }

  private requireWorkingCopy(): IWorkingCopy {
    const workingCopy = this.workingCopySlot.value;
    assertDefined(workingCopy, new ReferenceError("Document editor pane has no active working copy"));
    return workingCopy;
  }

  private requireInput(): EditorInput {
    const input = this.input;
    assertDefined(input, new ReferenceError("DocumentEditorPane has no active input"));
    return input;
  }

  private isReadOnly(): boolean {
    return this.input?.readOnly === true;
  }
}

function createFallbackInlineNode(ownerDocument: Document, node: DocumentNode): HTMLElement {
  const element = ownerDocument.createElement("span");
  element.className = "zeta-document-inline-node";
  const label = node.attrs.label;
  element.textContent = typeof label === "string" && label.length > 0 ? label : `[${node.type}]`;
  return element;
}

function findNode(document: DocumentNode, id: string): DocumentNode | undefined {
  if (document.id === id) return document;
  for (const child of document.content) {
    const nested = findNode(child, id);
    if (nested) return nested;
  }
  return undefined;
}

function normalizeDocumentNodeView(value: HTMLElement | DocumentNodeView, nodeType: string): DocumentNodeView {
  if (!value || typeof value !== "object") throw new TypeError(`Gamma node view '${nodeType}' must return an HTMLElement or node view handle`);
  if ("nodeType" in value) {
    if (value.nodeType !== 1) throw new TypeError(`Gamma node view '${nodeType}' must return an HTMLElement`);
    return { element: value as HTMLElement };
  }
  if (!("element" in value) || !value.element || value.element.nodeType !== 1) throw new TypeError(`Gamma node view '${nodeType}' must return an HTMLElement or node view handle`);
  const handle = value as DocumentNodeView;
  if (handle.update !== undefined && typeof handle.update !== "function") throw new TypeError(`Gamma node view '${nodeType}' update must be a function`);
  if (handle.dispose !== undefined && typeof handle.dispose !== "function") throw new TypeError(`Gamma node view '${nodeType}' dispose must be a function`);
  return handle;
}

function isInlineNodeInBlock(document: DocumentNode, blockId: string, nodeId: string): boolean {
  const block = findNode(document, blockId);
  return (block?.type === "paragraph" || block?.type === "heading") && block.content.some(child => child.id === nodeId && child.text === undefined);
}

function findTextArea(container: HTMLDivElement, blockId: string): HTMLTextAreaElement | undefined {
  for (const textarea of container.querySelectorAll<HTMLTextAreaElement>("textarea")) {
    if (textarea.dataset.blockId === blockId) return textarea;
  }
  return undefined;
}

function createTextareaTextSelection(document: DocumentNode, blockId: string, textarea: HTMLTextAreaElement): TextSelection | undefined {
  const block = findNode(document, blockId);
  const textNode = block?.content.length === 1 && block.content[0]?.text !== undefined ? block.content[0] : undefined;
  if (!textNode) return undefined;
  const start = Math.max(0, Math.min(textNode.text!.length, textarea.selectionStart ?? textarea.value.length));
  const end = Math.max(start, Math.min(textNode.text!.length, textarea.selectionEnd ?? start));
  return textSelection({ nodeId: textNode.id, offset: start }, { nodeId: textNode.id, offset: end });
}

function findBlockEditor(container: HTMLDivElement, blockId: string): HTMLElement | undefined {
  for (const editor of container.querySelectorAll<HTMLElement>("textarea, div.zeta-document-rich-text-input")) {
    if (editor.dataset.blockId === blockId) return editor;
  }
  return undefined;
}

function findFirstEditableBlock(node: DocumentNode): DocumentNode | undefined {
  if (node.type === "paragraph" || node.type === "heading" || node.type === "codeBlock") return node;
  for (const child of node.content) {
    const block = findFirstEditableBlock(child);
    if (block) return block;
  }
  return undefined;
}

function findFirstTextNode(node: DocumentNode): DocumentNode | undefined {
  if (node.text !== undefined) return node;
  for (const child of node.content) {
    const text = findFirstTextNode(child);
    if (text) return text;
  }
  return undefined;
}

function findImageClipboardFile(dataTransfer: DataTransfer | null | undefined): File | undefined {
  for (const file of Array.from(dataTransfer?.files ?? [])) {
    if (file.type.startsWith("image/")) return file;
  }
  for (const item of Array.from(dataTransfer?.items ?? [])) {
    if (item.kind !== "file" || !item.type.startsWith("image/")) continue;
    const file = item.getAsFile();
    if (file) return file;
  }
  return undefined;
}

function readBlobAsDataUrl(blob: Blob, ownerDocument: Document): Promise<string> {
  const FileReaderConstructor = ownerDocument.defaultView?.FileReader;
  if (!FileReaderConstructor) return Promise.reject(new Error("FileReader is not available"));
  return new Promise((resolve, reject) => {
    const reader = new FileReaderConstructor();
    reader.addEventListener("load", () => {
      if (typeof reader.result === "string") resolve(reader.result);
      else reject(new Error("Image clipboard data did not produce a data URL"));
    });
    reader.addEventListener("error", () => reject(reader.error ?? new Error("Unable to read image clipboard data")));
    reader.readAsDataURL(blob);
  });
}

function historyShortcut(event: KeyboardEvent): "undo" | "redo" | undefined {
  if ((!event.metaKey && !event.ctrlKey) || event.altKey) return undefined;
  const key = event.key.toLowerCase();
  if (key === "z") return event.shiftKey ? "redo" : "undo";
  if (key === "y" && !event.shiftKey) return "redo";
  return undefined;
}

function usesRichTextSurface(node: DocumentNode): boolean {
  const textNodes = node.content.filter(child => child.text !== undefined);
  return node.content.some(child => child.text === undefined) || textNodes.length > 1 || textNodes.some(child => child.marks.length > 0);
}

interface ViewDecoration {
  readonly from: number;
  readonly to: number;
  readonly decoration: DocumentDecoration;
}

interface DocumentComposition {
  readonly model: DocumentModel;
  readonly blockId: string;
  readonly element: HTMLTextAreaElement | HTMLDivElement;
  readonly selection: TextSelection;
  readonly baseText: string;
  readonly version: number;
}

function readCompositionText(element: HTMLTextAreaElement | HTMLDivElement): string {
  return element.tagName === "TEXTAREA" ? (element as HTMLTextAreaElement).value : element.textContent ?? "";
}

function findCompositionDiff(before: string, after: string): { readonly from: number; readonly to: number; readonly text: string } | undefined {
  let from = 0;
  while (from < before.length && from < after.length && before[from] === after[from]) from += 1;
  let beforeEnd = before.length;
  let afterEnd = after.length;
  while (beforeEnd > from && afterEnd > from && before[beforeEnd - 1] === after[afterEnd - 1]) {
    beforeEnd -= 1;
    afterEnd -= 1;
  }
  if (from === beforeEnd && from === afterEnd) return undefined;
  return { from, to: beforeEnd, text: after.slice(from, afterEnd) };
}

function findSingleTextNodeInBlock(document: DocumentNode, blockId: string): DocumentNode | undefined {
  const block = findNode(document, blockId);
  if (!block) return undefined;
  const textNodes = block.content.filter(child => child.text !== undefined);
  return textNodes.length === 1 ? textNodes[0] : undefined;
}

function resolveViewDecorations(model: DocumentModel): readonly ViewDecoration[] {
  const result: ViewDecoration[] = [];
  for (const source of model.getPluginDecorations()) {
    for (const decoration of source.set.decorations) {
      try {
        const from = documentPointToPosition(model.document, model.schema, decoration.from);
        const to = documentPointToPosition(model.document, model.schema, decoration.to);
        if (from === to) continue;
        result.push({ from: Math.min(from, to), to: Math.max(from, to), decoration });
      } catch {
        // A stale plugin range is ignored until the plugin maps or replaces it.
      }
    }
  }
  return Object.freeze(result);
}

function hasRenderableDecoration(model: DocumentModel, node: DocumentNode, decorations: readonly ViewDecoration[]): boolean {
  for (const child of node.content) {
    if (child.text === undefined || child.text.length === 0) continue;
    const start = documentPointToPosition(model.document, model.schema, { nodeId: child.id, offset: 0 });
    const end = documentPointToPosition(model.document, model.schema, { nodeId: child.id, offset: child.text.length });
    if (decorations.some(decoration => decoration.to > start && decoration.from < end)) return true;
  }
  return false;
}

function applyViewDecorations(element: HTMLElement, decorations: readonly DocumentDecoration[]): void {
  if (decorations.length === 0) return;
  element.classList.add("zeta-document-decoration");
  element.dataset.decorationIds = decorations.map(decoration => decoration.id).join(" ");
  for (const decoration of decorations) {
    for (const className of decoration.className?.split(/\s+/) ?? []) {
      if (/^-?[A-Za-z_][A-Za-z0-9_-]*$/.test(className)) element.classList.add(className);
    }
    for (const [key, value] of Object.entries(decoration.attrs)) {
      if (!/^data-[a-z0-9_-]+$/i.test(key) || (typeof value !== "string" && typeof value !== "number" && typeof value !== "boolean")) continue;
      element.setAttribute(key, String(value));
    }
  }
}

function nodeElementTagName(node: DocumentNode): string {
  switch (node.type) {
    case "heading": return "h2";
    case "blockquote": return "blockquote";
    case "bulletList": return "ul";
    case "orderedList": return "ol";
    case "listItem": return "li";
    case "table": return "table";
    case "tableRow": return "tr";
    case "tableCell": return "td";
    default: return "div";
  }
}

function findTextBlockId(root: DocumentNode, textNodeId: string | undefined): string | undefined {
  if (!textNodeId) return undefined;
  if ((root.type === "paragraph" || root.type === "heading") && root.content.some(child => child.id === textNodeId)) return root.id;
  for (const child of root.content) {
    const blockId = findTextBlockId(child, textNodeId);
    if (blockId) return blockId;
  }
  return undefined;
}

function findTextBlockContainingNode(root: DocumentNode, nodeId: string): string | undefined {
  if ((root.type === "paragraph" || root.type === "heading") && containsDocumentNode(root, nodeId)) return root.id;
  for (const child of root.content) {
    const blockId = findTextBlockContainingNode(child, nodeId);
    if (blockId) return blockId;
  }
  return undefined;
}

function findListForBlock(root: DocumentNode, blockId: string): DocumentNode | undefined {
  for (const child of root.content) {
    if ((child.type === "bulletList" || child.type === "orderedList") && containsDocumentNode(child, blockId)) return child;
    const list = findListForBlock(child, blockId);
    if (list) return list;
  }
  return undefined;
}

function findBlockquoteForBlock(root: DocumentNode, blockId: string): DocumentNode | undefined {
  for (const child of root.content) {
    if (child.type === "blockquote" && containsDocumentNode(child, blockId)) return child;
    const blockquote = findBlockquoteForBlock(child, blockId);
    if (blockquote) return blockquote;
  }
  return undefined;
}

function isTextSelectionMarked(block: DocumentNode, selection: TextSelection, markType: string, storedMarks?: readonly DocumentMark[]): boolean {
  const anchorIndex = block.content.findIndex(child => child.id === selection.anchor.nodeId && child.text !== undefined);
  const headIndex = block.content.findIndex(child => child.id === selection.head.nodeId && child.text !== undefined);
  if (anchorIndex < 0 || headIndex < 0) return false;
  const anchorNode = block.content[anchorIndex]!;
  const headNode = block.content[headIndex]!;
  if (selection.anchor.offset > anchorNode.text!.length || selection.head.offset > headNode.text!.length) return false;
  if (anchorIndex === headIndex && selection.anchor.offset === selection.head.offset) {
    return (storedMarks ?? anchorNode.marks).some(mark => mark.type === markType);
  }
  const forward = anchorIndex < headIndex || (anchorIndex === headIndex && selection.anchor.offset <= selection.head.offset);
  const startIndex = forward ? anchorIndex : headIndex;
  const endIndex = forward ? headIndex : anchorIndex;
  const startOffset = forward ? selection.anchor.offset : selection.head.offset;
  const endOffset = forward ? selection.head.offset : selection.anchor.offset;
  let selectedText = false;
  for (let index = startIndex; index <= endIndex; index += 1) {
    const node = block.content[index]!;
    if (node.text === undefined) return false;
    const from = index === startIndex ? startOffset : 0;
    const to = index === endIndex ? endOffset : node.text.length;
    if (to <= from) continue;
    selectedText = true;
    if (!node.marks.some(mark => mark.type === markType)) return false;
  }
  return selectedText;
}

interface DomTextSelection {
  readonly blockId: string;
  readonly nodeId: string;
  readonly selection: TextSelection;
}

function readDocumentTextSelection(container: HTMLDivElement, includeCollapsed = false): DomTextSelection | undefined {
  const selection = container.ownerDocument.getSelection();
  if (!selection || selection.rangeCount === 0 || (!includeCollapsed && selection.isCollapsed)) return undefined;
  const anchorRun = findInlineRun(container, selection.anchorNode);
  const headRun = findInlineRun(container, selection.focusNode);
  const anchorEditor = anchorRun ? findRichTextEditor(container, anchorRun) : undefined;
  const headEditor = headRun ? findRichTextEditor(container, headRun) : undefined;
  if (!anchorRun || !headRun || !anchorEditor || !headEditor || !anchorRun.dataset.textNodeId || !headRun.dataset.textNodeId || !anchorEditor.dataset.blockId) return undefined;
  const anchorOffset = offsetWithinInlineRun(container, anchorRun, selection.anchorNode, selection.anchorOffset);
  const headOffset = offsetWithinInlineRun(container, headRun, selection.focusNode, selection.focusOffset);
  return {
    blockId: anchorEditor.dataset.blockId,
    nodeId: anchorRun.dataset.textNodeId,
    selection: textSelection({ nodeId: anchorRun.dataset.textNodeId, offset: anchorOffset }, { nodeId: headRun.dataset.textNodeId, offset: headOffset }),
  };
}

function readInlineTextSelection(editor: HTMLDivElement, includeCollapsed = false): { nodeId: string; selection: TextSelection } | undefined {
  const selection = readDocumentTextSelection(editor, includeCollapsed);
  if (!selection || selection.blockId !== editor.dataset.blockId) return undefined;
  return { nodeId: selection.nodeId, selection: selection.selection };
}

function findInlineRun(editor: HTMLDivElement, node: Node | null): HTMLElement | undefined {
  let current = node;
  while (current && current !== editor) {
    if (current.nodeType === 1) {
      const element = current as HTMLElement;
      if (element.dataset.textNodeId) return element;
    }
    current = current.parentNode;
  }
  return undefined;
}

function findRichTextEditor(container: HTMLDivElement, node: Node): HTMLDivElement | undefined {
  let current: Node | null = node;
  while (current) {
    if (current.nodeType === 1) {
      const element = current as HTMLElement;
      if (element.classList.contains("zeta-document-rich-text-input")) return element as HTMLDivElement;
    }
    if (current === container) break;
    current = current.parentNode;
  }
  return container.classList.contains("zeta-document-rich-text-input") ? container : undefined;
}

function offsetWithinInlineRun(editor: HTMLDivElement, run: HTMLElement, node: Node | null, offset: number): number {
  if (!node) return 0;
  const nodeId = run.dataset.textNodeId;
  let offsetBefore = 0;
  if (nodeId) {
    for (const candidate of editor.querySelectorAll<HTMLElement>("[data-text-node-id]")) {
      if (candidate.dataset.textNodeId !== nodeId) continue;
      if (candidate === run) break;
      offsetBefore += candidate.textContent?.length ?? 0;
    }
  }
  const range = editor.ownerDocument.createRange();
  range.selectNodeContents(run);
  range.setEnd(node, offset);
  return offsetBefore + range.toString().length;
}

function setInlineTextSelection(editor: HTMLDivElement, selection: TextSelection): void {
  const anchorPosition = findInlineRunAtOffset(editor, selection.anchor.nodeId, selection.anchor.offset);
  const headPosition = findInlineRunAtOffset(editor, selection.head.nodeId, selection.head.offset);
  if (!anchorPosition || !headPosition) return;
  const anchorRun = anchorPosition.run;
  const headRun = headPosition.run;
  const anchorText = anchorRun.firstChild ?? anchorRun;
  const headText = headRun.firstChild ?? headRun;
  const anchorOffset = anchorPosition.offset;
  const headOffset = headPosition.offset;
  const domSelection = editor.ownerDocument.getSelection();
  if (domSelection?.setBaseAndExtent) {
    domSelection.removeAllRanges();
    try {
      domSelection.setBaseAndExtent(anchorText, anchorOffset, headText, headOffset);
      return;
    } catch {
      // Fall back to a normalized range for DOM implementations without full contenteditable support.
    }
  }
  const range = editor.ownerDocument.createRange();
  const forward = anchorRun === headRun ? anchorOffset <= headOffset : (anchorRun.compareDocumentPosition(headRun) & 4) !== 0;
  if (forward) {
    range.setStart(anchorText, anchorOffset);
    range.setEnd(headText, headOffset);
  } else {
    range.setStart(headText, headOffset);
    range.setEnd(anchorText, anchorOffset);
  }
  domSelection?.removeAllRanges();
  domSelection?.addRange(range);
}

function findInlineRunAtOffset(editor: HTMLDivElement, nodeId: string, offset: number): { readonly run: HTMLElement; readonly offset: number } | undefined {
  const runs = [...editor.querySelectorAll<HTMLElement>("[data-text-node-id]")].filter(run => run.dataset.textNodeId === nodeId);
  if (runs.length === 0) return undefined;
  let remaining = Math.max(0, offset);
  for (let index = 0; index < runs.length; index += 1) {
    const run = runs[index]!;
    const length = run.textContent?.length ?? 0;
    if (remaining <= length || index === runs.length - 1) return { run, offset: Math.min(remaining, length) };
    remaining -= length;
  }
  return undefined;
}

function createInlineBoundaryCommand(document: DocumentNode, blockId: string, textNodeId: string, direction: "backward" | "forward"): DocumentCommand | undefined {
  const block = findNode(document, blockId);
  if (!block) return undefined;
  const index = block.content.findIndex(child => child.id === textNodeId);
  const adjacent = block.content[index + (direction === "backward" ? -1 : 1)];
  if (index >= 0 && adjacent?.text !== undefined) return createJoinAdjacentTextRunCommand(document, blockId, textNodeId, direction);
  if (index >= 0 && adjacent && adjacent.text === undefined) return createDeleteAdjacentInlineNodeCommand(document, blockId, textNodeId, direction);
  const blockCommand = createJoinAdjacentBlockCommand(document, blockId, textNodeId, direction);
  if (blockCommand) return blockCommand;
  const location = findDocumentNode(document, blockId);
  const parent = location?.parent;
  if (parent?.type !== "listItem") return undefined;
  const blockIndex = parent.content.findIndex(child => child.id === blockId);
  const atBoundary = direction === "backward" ? blockIndex === 0 : blockIndex === parent.content.length - 1;
  return atBoundary ? createJoinAdjacentListItemCommand(document, parent.id, blockId, direction) : undefined;
}

function createParagraphSplitCommand(schema: DocumentSchema, document: DocumentNode, paragraphId: string, textNodeId: string, offset: number): DocumentCommand | undefined {
  const location = findDocumentNode(document, paragraphId);
  if (location?.parent?.type === "listItem") {
    if (!textNodeId) return createExitEmptyListItemCommand(schema, document, location.parent.id, paragraphId);
    return createSplitListItemCommand(schema, document, location.parent.id, paragraphId, textNodeId, offset);
  }
  if (!textNodeId) return undefined;
  return createSplitBlockCommand(schema, document, paragraphId, textNodeId, offset);
}

function createListItemIndentationForBlock(schema: DocumentSchema, document: DocumentNode, paragraphId: string, direction: "in" | "out"): DocumentCommand | undefined {
  const location = findDocumentNode(document, paragraphId);
  if (location?.parent?.type !== "listItem") return undefined;
  return createListItemIndentationCommand(schema, document, location.parent.id, paragraphId, direction);
}

function createDeleteBoundaryCommand(model: DocumentModel, blockId: string, selection: TextSelection, direction: "backward" | "forward"): DocumentCommand | undefined {
  if (!isCollapsedTextSelection(selection)) return createDeleteInlineSelectionCommand(model.schema, model.document, blockId, selection) ?? createReplaceTextCommand(model.schema, model.document, blockId, selection, "");
  const point = selection.anchor;
  const textNode = findNode(model.document, point.nodeId);
  if (!textNode || textNode.text === undefined) return undefined;
  if (direction === "backward" && point.offset > 0) {
    return createReplaceTextCommand(model.schema, model.document, blockId, textSelection({ nodeId: point.nodeId, offset: point.offset - 1 }, point), "");
  }
  if (direction === "forward" && point.offset < textNode.text.length) {
    return createReplaceTextCommand(model.schema, model.document, blockId, textSelection(point, { nodeId: point.nodeId, offset: point.offset + 1 }), "");
  }
  return createInlineBoundaryCommand(model.document, blockId, point.nodeId, direction);
}

function isCollapsedTextSelection(selection: TextSelection): boolean {
  return selection.anchor.nodeId === selection.head.nodeId && selection.anchor.offset === selection.head.offset;
}

function documentInsertionMarks(model: DocumentModel, selection: DocumentSelection | undefined): readonly DocumentMark[] | undefined {
  if (model.storedMarks !== undefined) return model.storedMarks;
  if (!selection || selection.kind !== "text" || !isCollapsedTextSelection(selection)) return undefined;
  const node = findNode(model.document, selection.anchor.nodeId);
  return node?.text === undefined ? undefined : node.marks;
}

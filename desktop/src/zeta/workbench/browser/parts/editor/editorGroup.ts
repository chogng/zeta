import { addDisposableListener } from "../../../../base/browser/dom.js";
import { Dimension, type IDimension } from "../../../../base/browser/geometry.js";
import { DisposableOwner, setDisposableOwner } from "../../../../base/common/lifecycle.js";
import type { IKeybindingService } from "../../../../platform/keybinding/common/keybinding.js";
import type { IConfigurationService } from "../../../../platform/configuration/common/configuration.js";
import type { EditorInput, EditorOpenOptions } from "./editorInput.js";
import { type IEditorPane, EditorPaneVisibility } from "./editorPane.js";
import { EditorPaneRegistry } from "./editorRegistry.js";
import { EditorGroupWatermark } from "./editorGroupWatermark.js";
import { editorInputKey, type EditorTabDescriptor } from "./editorTabsControl.js";
import { EditorTitleControl, type EditorTitleActions } from "./editorTitleControl.js";

/** Operations and state owned independently by one EditorGroup. */
export interface IEditorGroup {
  readonly element: HTMLElement;
  readonly inputs: readonly EditorInput[];
  readonly activeInput: EditorInput | undefined;
  readonly activePane: IEditorPane | undefined;

  openEditor(
    input: EditorInput,
    options?: EditorOpenOptions,
  ): Promise<IEditorPane>;
  activateEditor(input: EditorInput): IEditorPane;
  closeEditor(input: EditorInput): void;
  setContent(content: Element): void;
  layout(dimension: IDimension): void;
  focus(): void;
}

/** Construction inputs for one independently navigable EditorGroup. */
export interface EditorGroupOptions {
  readonly ownerDocument: Document;
  readonly registry: EditorPaneRegistry;
  readonly configurationService?: IConfigurationService;
  readonly keybindingService?: IKeybindingService;
  readonly titleActions?: EditorTitleActions;
  readonly onDidActivate?: () => void;
}

interface EditorGroupEntry extends EditorTabDescriptor {
  paneInstance: EditorPaneInstance;
  input: EditorInput;
}

/**
 * Owns an ordered set of Editor inputs, their Pane lifetimes, and title UI.
 *
 * EditorPart owns group layout. This class owns only the behavior that remains
 * independent when the Part later contains multiple split groups.
 */
export class EditorGroup extends DisposableOwner implements IEditorGroup {
  readonly element: HTMLElement;
  readonly #contentElement: HTMLDivElement;
  readonly #registry: EditorPaneRegistry;
  readonly #configurationService: IConfigurationService | undefined;
  readonly #titleControl: EditorTitleControl;
  readonly #watermarkElement: HTMLElement;
  readonly #entries: EditorGroupEntry[] = [];
  #activeEntry: EditorGroupEntry | undefined;
  #ordinaryContent: Element | undefined;
  #dimension: IDimension = Dimension.Zero;
  #openSequence = 0;
  #pendingPane: EditorPaneInstance | undefined;

  constructor(options: EditorGroupOptions) {
    super();
    this.#registry = options.registry;
    this.#configurationService = options.configurationService;
    this.element = options.ownerDocument.createElement("section");
    this.element.className = "zeta-editor-group";
    this.element.setAttribute("aria-label", "Editor group");
    if (options.onDidActivate) {
      this.own(addDisposableListener(this.element, "focusin", () => {
        options.onDidActivate?.();
      }));
    }
    this.#titleControl = this.own(new EditorTitleControl(
      options.ownerDocument,
      {
        activate: (input) => {
          this.#activateEntry(this.#requireEntry(input), true);
        },
        close: (input) => this.closeEditor(input),
      },
      options.titleActions,
    ));
    this.#contentElement = options.ownerDocument.createElement("div");
    this.#contentElement.className = "zeta-editor-group-content";
    const watermark = options.keybindingService
      ? this.own(new EditorGroupWatermark(
        options.ownerDocument,
        options.keybindingService,
      ))
      : undefined;
    this.#watermarkElement = watermark?.element ??
      options.ownerDocument.createElement("div");
    this.#watermarkElement.classList.add("zeta-editor-group-watermark");
    this.#contentElement.append(this.#watermarkElement);
    this.element.append(
      this.#titleControl.element,
      this.#contentElement,
    );
    this.defer(() => {
      this.#cancelPendingOpen();
      for (const entry of this.#entries) entry.paneInstance.dispose();
      this.#entries.length = 0;
    });
    this.defer(() => this.element.remove());
    this.#renderChrome();
  }

  get inputs(): readonly EditorInput[] {
    return this.#entries.map(({ input }) => input);
  }

  get activeInput(): EditorInput | undefined {
    return this.#activeEntry?.input;
  }

  get activePane(): IEditorPane | undefined {
    return this.#activeEntry?.paneInstance.pane;
  }

  async openEditor(
    input: EditorInput,
    options: EditorOpenOptions = {},
  ): Promise<IEditorPane> {
    const sequence = ++this.#openSequence;
    this.#cancelPendingOpen();
    const descriptor = this.#registry.resolve(input, options);
    const existing = this.#entry(input);
    if (existing?.paneInstance.pane.id === descriptor.id) {
      existing.input = input;
      this.#activateEntry(existing, false);
      return existing.paneInstance.pane;
    }

    const pane = descriptor.create({
      ownerDocument: this.element.ownerDocument,
      configurationService: this.#configurationService,
    });
    if (pane.id !== descriptor.id) {
      pane.dispose();
      throw new TypeError(
        `Editor pane factory '${descriptor.id}' created '${pane.id}'`,
      );
    }
    const paneInstance = new EditorPaneInstance(
      pane,
      this.element.ownerDocument,
    );
    setDisposableOwner(paneInstance, this);
    this.#pendingPane = paneInstance;
    this.#contentElement.append(paneInstance.element);
    try {
      pane.create(paneInstance.element);
      paneInstance.setVisible(EditorPaneVisibility.Hidden);
      await pane.setInput(input, paneInstance.signal);
    } catch (error) {
      if (this.#pendingPane === paneInstance) {
        this.#pendingPane = undefined;
      }
      paneInstance.dispose();
      if (sequence !== this.#openSequence) {
        throw new EditorOpenSupersededError(input);
      }
      throw error;
    }

    if (
      sequence !== this.#openSequence ||
      this.#pendingPane !== paneInstance
    ) {
      paneInstance.dispose();
      throw new EditorOpenSupersededError(input);
    }
    this.#pendingPane = undefined;

    let entry: EditorGroupEntry = {
      input,
      panelId: paneInstance.panelId,
      tabId: paneInstance.tabId,
      paneInstance,
    };
    if (existing) {
      const index = this.#entries.indexOf(existing);
      existing.paneInstance.setVisible(EditorPaneVisibility.Hidden);
      existing.paneInstance.dispose();
      if (this.#activeEntry === existing) this.#activeEntry = undefined;
      this.#entries[index] = entry;
    } else {
      this.#entries.push(entry);
    }
    this.#ordinaryContent = undefined;
    this.#activateEntry(entry, false);
    return pane;
  }

  activateEditor(input: EditorInput): IEditorPane {
    const entry = this.#requireEntry(input);
    this.#activateEntry(entry, false);
    return entry.paneInstance.pane;
  }

  closeEditor(input: EditorInput): void {
    const index = this.#entries.findIndex(
      (candidate) => editorInputKey(candidate.input) === editorInputKey(input),
    );
    if (index < 0) return;
    const [entry] = this.#entries.splice(index, 1);
    if (!entry) return;
    const wasActive = this.#activeEntry === entry;
    if (wasActive) {
      this.#activeEntry = undefined;
      entry.paneInstance.setVisible(EditorPaneVisibility.Hidden);
    }
    entry.paneInstance.dispose();
    if (wasActive) {
      const next = this.#entries[index] ?? this.#entries[index - 1];
      if (next) this.#activateEntry(next, true);
    }
    this.#renderContent();
    this.#renderChrome();
  }

  setContent(content: Element): void {
    this.#openSequence += 1;
    this.#cancelPendingOpen();
    for (const entry of this.#entries) {
      entry.paneInstance.setVisible(EditorPaneVisibility.Hidden);
      entry.paneInstance.dispose();
    }
    this.#entries.length = 0;
    this.#activeEntry = undefined;
    this.#ordinaryContent = content;
    this.#renderContent();
    this.#renderChrome();
  }

  layout(dimension: IDimension): void {
    this.#dimension = new Dimension(
      dimension.width,
      Math.max(0, dimension.height - EditorTitleControl.HEIGHT),
    );
    this.activePane?.layout(this.#dimension);
  }

  focus(): void {
    this.activePane?.focus();
  }

  #activateEntry(entry: EditorGroupEntry, focus: boolean): void {
    if (this.#activeEntry !== entry) {
      this.#activeEntry?.paneInstance.setVisible(EditorPaneVisibility.Hidden);
      this.#activeEntry = entry;
    }
    this.#ordinaryContent = undefined;
    this.#renderContent();
    entry.paneInstance.pane.layout(this.#dimension);
    entry.paneInstance.setVisible(EditorPaneVisibility.Visible);
    this.#renderChrome();
    if (focus) entry.paneInstance.pane.focus();
  }

  #renderContent(): void {
    const children: Element[] = [];
    if (this.#ordinaryContent) {
      children.push(this.#ordinaryContent);
    } else {
      this.#watermarkElement.hidden = this.#entries.length > 0;
      children.push(
        this.#watermarkElement,
        ...this.#entries.map(({ paneInstance }) => paneInstance.element),
      );
    }
    if (this.#pendingPane) children.push(this.#pendingPane.element);
    this.#contentElement.replaceChildren(...children);
  }

  #renderChrome(): void {
    this.#titleControl.setEditors(this.#entries, this.activeInput);
  }

  #entry(input: EditorInput): EditorGroupEntry | undefined {
    const key = editorInputKey(input);
    return this.#entries.find(
      (candidate) => editorInputKey(candidate.input) === key,
    );
  }

  #requireEntry(input: EditorInput): EditorGroupEntry {
    const entry = this.#entry(input);
    if (!entry) {
      throw new RangeError(
        `Editor is not open in this group: ${input.resource}`,
      );
    }
    return entry;
  }

  #cancelPendingOpen(): void {
    const pending = this.#pendingPane;
    this.#pendingPane = undefined;
    pending?.dispose();
  }
}

let editorPaneId = 0;

class EditorPaneInstance extends DisposableOwner {
  readonly element: HTMLDivElement;
  readonly signal: AbortSignal;
  readonly panelId: string;
  readonly tabId: string;

  constructor(
    readonly pane: IEditorPane,
    ownerDocument: Document,
  ) {
    super();
    const id = ++editorPaneId;
    this.panelId = `zeta-editor-pane-${id}`;
    this.tabId = `zeta-editor-tab-${id}`;
    const AbortControllerConstructor =
      ownerDocument.defaultView?.AbortController ?? AbortController;
    const abortController = new AbortControllerConstructor();
    this.signal = abortController.signal;
    this.element = ownerDocument.createElement("div");
    this.element.id = this.panelId;
    this.element.className = "zeta-editor-pane-host";
    this.element.setAttribute("role", "tabpanel");
    this.element.setAttribute("aria-labelledby", this.tabId);
    this.defer(() => this.element.remove());
    this.own(pane);
    this.defer(() => pane.clearInput());
    this.defer(() => pane.setVisible(EditorPaneVisibility.Hidden));
    this.defer(() => abortController.abort());
  }

  setVisible(visibility: EditorPaneVisibility): void {
    this.element.hidden = visibility === EditorPaneVisibility.Hidden;
    this.pane.setVisible(visibility);
  }
}

export class EditorOpenSupersededError extends Error {
  constructor(readonly input: EditorInput) {
    super(`Editor opening was superseded: ${input.resource}`);
    this.name = "EditorOpenSupersededError";
  }
}

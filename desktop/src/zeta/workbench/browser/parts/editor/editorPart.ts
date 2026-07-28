import "./editorpart.css";
import {
  Dimension,
  type IDimension,
} from "../../../../base/browser/geometry.js";
import {
  DisposableOwner,
  DisposableSlot,
} from "../../../../base/common/lifecycle.js";
import {
  createServiceIdentifier,
} from "../../../../platform/instantiation/common/instantiation.js";
import { WorkbenchPart } from "../../part.js";
import type {
  EditorInput,
  EditorOpenOptions,
} from "./editorInput.js";
import {
  type IEditorPane,
  EditorPaneVisibility,
} from "./editorPane.js";
import {
  EditorPaneRegistry,
  EditorPanes,
} from "./editorRegistry.js";
import {
  EditorGroupWatermark,
} from "./editorGroupWatermark.js";
import type {
  IKeybindingService,
} from "../../../../platform/keybinding/common/keybinding.js";

/** Editor-region operations available to Workbench contributions. */
export interface IEditorPart {
  readonly element: HTMLElement;
  readonly activeInput: EditorInput | undefined;
  readonly activePane: IEditorPane | undefined;

  openEditor(
    input: EditorInput,
    options?: EditorOpenOptions,
  ): Promise<IEditorPane>;
  setContent(content: Element): void;
  layout(dimension: IDimension): void;
  focus(): void;
}

export const IEditorPart =
  createServiceIdentifier<IEditorPart>("editorPart");

/** Named collaborators used to construct the editor region. */
export interface IEditorPartOptions {
  readonly keybindingService?: IKeybindingService;
  readonly registry?: EditorPaneRegistry;
}

/** The central content region that hosts the active workbench editor or view. */
export class EditorPart extends WorkbenchPart implements IEditorPart {
  readonly #activeSession =
    this.own(new DisposableSlot<EditorPaneSession>());
  readonly #registry: EditorPaneRegistry;
  #activeInput: EditorInput | undefined;
  #dimension: IDimension = Dimension.Zero;
  #openSequence = 0;
  #pendingSession: EditorPaneSession | undefined;

  override get minimumWidth(): number { return 120; }
  override get minimumHeight(): number { return 84; }

  constructor(
    ownerDocument: Document,
    options: IEditorPartOptions = {},
  ) {
    super("editor", ownerDocument);
    this.#registry = options.registry ?? EditorPanes;
    this.element.setAttribute("aria-label", "Editor");
    if (options.keybindingService) {
      const watermark = this.own(new EditorGroupWatermark(
        ownerDocument,
        options.keybindingService,
      ));
      this.contentElement.append(watermark.element);
    }
    const ResizeObserverConstructor =
      ownerDocument.defaultView?.ResizeObserver;
    if (ResizeObserverConstructor) {
      const observer = new ResizeObserverConstructor(([entry]) => {
        if (!entry) return;
        const borderBox = entry.borderBoxSize[0];
        this.layout(new Dimension(
          borderBox?.inlineSize ?? entry.contentRect.width,
          borderBox?.blockSize ?? entry.contentRect.height,
        ));
      });
      observer.observe(this.contentElement, { box: "border-box" });
      this.defer(() => observer.disconnect());
    }
    this.defer(() => this.#cancelPendingOpen());
  }

  get activeInput(): EditorInput | undefined {
    return this.#activeInput;
  }

  get activePane(): IEditorPane | undefined {
    return this.#activeSession.value?.pane;
  }

  async openEditor(
    input: EditorInput,
    options: EditorOpenOptions = {},
  ): Promise<IEditorPane> {
    const sequence = ++this.#openSequence;
    this.#cancelPendingOpen();
    const descriptor = this.#registry.resolve(input, options);
    const pane = descriptor.create({
      ownerDocument: this.element.ownerDocument,
    });
    if (pane.id !== descriptor.id) {
      pane.dispose();
      throw new TypeError(
        `Editor pane factory '${descriptor.id}' created '${pane.id}'`,
      );
    }

    const session = new EditorPaneSession(
      pane,
      this.element.ownerDocument,
    );
    this.#pendingSession = session;
    this.contentElement.append(session.element);
    try {
      pane.create(session.element);
      pane.setVisible(EditorPaneVisibility.Hidden);
      await pane.setInput(input, session.signal);
    } catch (error) {
      if (this.#pendingSession === session) {
        this.#pendingSession = undefined;
      }
      session.dispose();
      if (sequence !== this.#openSequence) {
        throw new EditorOpenSupersededError(input);
      }
      throw error;
    }

    if (
      sequence !== this.#openSequence ||
      this.#pendingSession !== session
    ) {
      session.dispose();
      throw new EditorOpenSupersededError(input);
    }
    this.#pendingSession = undefined;

    const previous = this.#activeSession.value;
    previous?.pane.setVisible(EditorPaneVisibility.Hidden);
    this.contentElement.replaceChildren(session.element);
    this.#activeSession.replace(session);
    this.#activeInput = input;
    pane.layout(this.#dimension);
    pane.setVisible(EditorPaneVisibility.Visible);
    return pane;
  }

  setContent(content: Element): void {
    this.#openSequence += 1;
    this.#cancelPendingOpen();
    this.#activeInput = undefined;
    this.#activeSession.value?.pane.setVisible(
      EditorPaneVisibility.Hidden,
    );
    this.#activeSession.clear();
    this.contentElement.replaceChildren(content);
  }

  override layout(dimension: IDimension): void {
    this.#dimension = new Dimension(dimension.width, dimension.height);
    this.activePane?.layout(this.#dimension);
  }

  focus(): void {
    this.activePane?.focus();
  }

  #cancelPendingOpen(): void {
    const pending = this.#pendingSession;
    this.#pendingSession = undefined;
    pending?.dispose();
  }
}

class EditorPaneSession extends DisposableOwner {
  readonly element: HTMLDivElement;
  readonly signal: AbortSignal;

  constructor(
    readonly pane: IEditorPane,
    ownerDocument: Document,
  ) {
    super();
    const AbortControllerConstructor =
      ownerDocument.defaultView?.AbortController ?? AbortController;
    const abortController = new AbortControllerConstructor();
    this.signal = abortController.signal;
    this.element = ownerDocument.createElement("div");
    this.element.className = "zeta-editor-pane-host";
    this.defer(() => this.element.remove());
    this.own(pane);
    this.defer(() => pane.clearInput());
    this.defer(() => pane.setVisible(EditorPaneVisibility.Hidden));
    this.defer(() => abortController.abort());
  }
}

export class EditorOpenSupersededError extends Error {
  constructor(readonly input: EditorInput) {
    super(`Editor opening was superseded: ${input.resource}`);
    this.name = "EditorOpenSupersededError";
  }
}

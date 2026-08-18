import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import type { IContextKey, IContextKeyService } from "../../../../platform/contextkey/common/contextkey.js";
import type { IInstantiationService } from "../../../../platform/instantiation/common/instantiation.js";
import { FocusedViewContext } from "../../../common/contextkeys.js";
import type { IViewContainerDescriptor, IViewContainerModel, IViewDescriptor } from "../../../common/views.js";
import { ViewPane } from "./viewPane.js";
import { h } from "../../../../base/browser/dom.js";

/** Construction inputs for one browser view container. */
export interface ViewPaneContainerOptions {
  readonly viewContainer: IViewContainerDescriptor;
  readonly model: IViewContainerModel;
  readonly instantiationService: IInstantiationService;
  readonly contextKeyService: IContextKeyService;
  readonly onDidFailCreateView?: (
    error: unknown,
    viewId: string,
  ) => void;
}

/**
 * Browser host for the visible panes projected by one container model.
 */
export class ViewPaneContainer extends DisposableOwner {
  readonly element: HTMLElement;
  readonly id: string;
  readonly viewContainer: IViewContainerDescriptor;
  private readonly model: IViewContainerModel;
  private readonly instantiationService: IInstantiationService;
  private readonly focusedView: IContextKey<string>;
  private readonly onDidFailCreateView: (
    error: unknown,
    viewId: string,
  ) => void;
  private readonly _panes = new Map<string, ViewPaneItem>();
  private visible = true;

  constructor(container: HTMLElement, options: ViewPaneContainerOptions) {
    super();
    const ownerDocument = container.ownerDocument;
    const element = h(ownerDocument, "div");
    this.element = element;
    this.defer(() => element.remove());
    element.className = "zeta-view-pane-container";
    element.dataset.viewContainerId = options.viewContainer.id;
    container.append(element);
    this.id = options.viewContainer.id;
    this.viewContainer = options.viewContainer;
    this.model = options.model;
    this.instantiationService = options.instantiationService;
    this.onDidFailCreateView = options.onDidFailCreateView ??
      ((error, viewId) => {
        console.error(`Unable to create view pane '${viewId}'`, error);
      });
    this.focusedView = FocusedViewContext.bindTo(
      options.contextKeyService,
    );
    this.defer(() => {
      if (
        [...this._panes.values()].some(
          (item) => item.pane.id === this.focusedView.get(),
        )
      ) {
        this.focusedView.reset();
      }
      this._panes.clear();
    });
    this.own(this.model.onDidChangeVisibleViewDescriptors(() => {
      this.syncPanes();
    }));
    this.syncPanes();
  }

  get panes(): readonly ViewPane[] {
    return this.model.visibleViewDescriptors
      .map((descriptor) => this._panes.get(descriptor.id)?.pane)
      .filter((pane): pane is ViewPane => pane !== undefined);
  }

  getView(id: string): ViewPane | undefined {
    return this._panes.get(id)?.pane;
  }

  isVisible(): boolean {
    return this.visible;
  }

  setVisible(visible: boolean): void {
    if (this.visible === visible) return;
    this.visible = visible;
    this.element.hidden = !visible;
    for (const item of this._panes.values()) {
      item.pane.setVisible(visible);
    }
  }

  openView(id: string): ViewPane | undefined {
    if (!this.model.isVisible(id)) this.model.setVisible(id, true);
    return this._panes.get(id)?.pane;
  }

  focusView(id: string): boolean {
    const pane = this.openView(id);
    if (!pane) return false;
    pane.focus();
    return true;
  }

  focus(): void {
    this.panes[0]?.focus();
  }

  private syncPanes(): void {
    const desired = this.model.visibleViewDescriptors;
    const desiredIds = new Set(desired.map((view) => view.id));
    for (const [viewId, item] of this._panes) {
      if (desiredIds.has(viewId)) continue;
      this._panes.delete(viewId);
      item.dispose();
    }
    for (const descriptor of desired) {
      if (this._panes.has(descriptor.id)) continue;
      let pane: ViewPane;
      try {
        pane = this.createView(descriptor);
      } catch (error) {
        this.onDidFailCreateView(error, descriptor.id);
        continue;
      }
      pane.setVisible(this.visible);
      this._panes.set(
        descriptor.id,
        this.own(new ViewPaneItem(pane, this.focusedView)),
      );
    }
    this.element.replaceChildren(
      ...desired.flatMap((descriptor) => {
        const pane = this._panes.get(descriptor.id)?.pane;
        return pane ? [pane.element] : [];
      }),
    );
  }

  private createView(descriptor: IViewDescriptor): ViewPane {
    const view = this.instantiationService.createInstance(
      descriptor.ctorDescriptor,
      this.element,
      {
        id: descriptor.id,
        title: descriptor.title,
        collapsed: descriptor.collapsed,
      },
    );
    if (!(view instanceof ViewPane)) {
      throw new TypeError(
        `View constructor did not create a ViewPane: ${descriptor.id}`,
      );
    }
    if (view.id !== descriptor.id) {
      view.dispose();
      throw new Error(
        `View constructor returned '${view.id}' for '${descriptor.id}'`,
      );
    }
    return view;
  }
}

class ViewPaneItem extends DisposableOwner {
  constructor(
    readonly pane: ViewPane,
    focusedView: IContextKey<string>,
  ) {
    super();
    this.own(pane);
    this.own(pane.onDidFocus(() => focusedView.set(pane.id)));
    this.own(pane.onDidBlur(() => {
      if (focusedView.get() === pane.id) focusedView.reset();
    }));
    this.defer(() => {
      if (focusedView.get() === pane.id) focusedView.reset();
    });
  }
}

import { TabList } from "../../../../base/browser/ui/tablist/tabList.js";
import type { IAction } from "../../../../base/common/actions.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { LxIcon } from "../../../../base/common/lxicons.js";
import type { EditorInput } from "./editorInput.js";

/** One open Editor presented by an EditorTabsControl. */
export interface EditorTabDescriptor {
  readonly input: EditorInput;
  readonly panelId: string;
  readonly tabId: string;
}

/** Callbacks through which tabs request group-level mutations. */
export interface EditorTabsDelegate {
  activate(input: EditorInput): void;
  close(input: EditorInput): void;
}

/** Maps Editor inputs and lifecycle callbacks onto the shared TabList. */
export class EditorTabsControl extends DisposableOwner {
  readonly element: HTMLDivElement;
  readonly #delegate: EditorTabsDelegate;
  readonly #tabList: TabList<EditorInput>;

  constructor(ownerDocument: Document, delegate: EditorTabsDelegate) {
    super();
    this.#delegate = delegate;
    this.element = ownerDocument.createElement("div");
    this.element.className = "zeta-editor-tabs-control";
    this.#tabList = this.own(new TabList({
      ownerDocument,
      ariaLabel: "Open editors",
      onActivate: (input) => delegate.activate(input),
      onDelete: (input) => delegate.close(input),
    }));
    this.element.append(this.#tabList.element);
    this.defer(() => this.element.remove());
  }

  setEditors(
    editors: readonly EditorTabDescriptor[],
    activeInput: EditorInput | undefined,
  ): void {
    const activeKey = activeInput
      ? editorInputKey(activeInput)
      : undefined;
    this.#tabList.setTabs(
      editors.map((editor) => {
        const label = editorInputLabel(editor.input);
        return {
          id: editorInputKey(editor.input),
          value: editor.input,
          label,
          tooltip: editor.input.resource.toString(),
          tabId: editor.tabId,
          panelId: editor.panelId,
          actions: {
            ariaLabel: `${label} actions`,
            items: [
              closeEditorAction(editor.input, label, this.#delegate),
            ],
          },
        };
      }),
      activeKey,
    );
    this.element.hidden = editors.length === 0;
  }
}

function closeEditorAction(
  input: EditorInput,
  inputLabel: string,
  delegate: EditorTabsDelegate,
): IAction {
  const label = `Close ${inputLabel}`;
  return {
    id: "zeta.editor.close",
    label,
    tooltip: label,
    icon: LxIcon.close,
    enabled: true,
    run: () => delegate.close(input),
  };
}

export function editorInputKey(input: EditorInput): string {
  return input.resource.toString();
}

export function editorInputLabel(input: EditorInput): string {
  if (input.label?.trim()) return input.label;
  const path = decodeURIComponent(input.resource.path).replace(/\/+$/, "");
  const separator = path.lastIndexOf("/");
  return path.slice(separator + 1) || input.resource.toString();
}

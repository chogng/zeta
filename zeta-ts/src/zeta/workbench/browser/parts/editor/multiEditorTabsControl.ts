import "./media/multiEditorTabsControl.css";
import { DataTransfers } from "../../../../base/browser/dnd.js";
import { TabList } from "../../../../base/browser/ui/tablist/tabList.js";
import { containsExternalEditorDrop } from "./editorDropData.js";
import type { EditorInput } from "./editorInput.js";
import { EditorTabsControl, editorInputKey, type EditorTabDescriptor, type EditorTabsDelegate } from "./editorTabsControl.js";

const DRAG_OVER_ACTIVATE_DELAY = 1500;

/** Renders every open Editor in one reorderable tab list. */
export class MultiEditorTabsControl extends EditorTabsControl {
	private readonly tabList: TabList<EditorInput>;
	private previewedInput: EditorInput | undefined;

	constructor(container: HTMLElement, delegate: EditorTabsDelegate) {
		super(container);
		this.domNode.classList.add("zeta-multi-editor-tabs-control");
		this.tabList = this.own(new TabList(this.domNode, {
			ariaLabel: "Open editors",
			presentation: "inset",
			draggable: true,
			dragAndDrop: {
				canDrop: (event) => delegate.isDragging() || containsExternalEditorDrop(event),
				onDragStart: (input, event) => {
					event.dataTransfer?.setData(DataTransfers.Text, editorInputKey(input));
					if (event.dataTransfer) event.dataTransfer.effectAllowed = "move";
					delegate.startDrag(input);
				},
				onDragEnter: (_target, _position, event) => {
					this.previewedInput = undefined;
					if (event.dataTransfer) event.dataTransfer.dropEffect = delegate.isDragging() ? "move" : "copy";
				},
				onDragOver: (target, _position, event, duration) => {
					if (event.dataTransfer) event.dataTransfer.dropEffect = delegate.isDragging() ? "move" : "copy";
					if (target && duration >= DRAG_OVER_ACTIVATE_DELAY && target !== this.previewedInput) {
						this.previewedInput = target;
						delegate.preview(target);
					}
				},
				onDragLeave: () => {
					this.previewedInput = undefined;
				},
				onDrop: (target, position, event) => {
					this.previewedInput = undefined;
					event.stopPropagation();
					if (delegate.isDragging()) delegate.drop(target, position);
					else delegate.dropExternal(event, target, position);
				},
				onDragEnd: () => {
					this.previewedInput = undefined;
					delegate.endDrag();
				},
			},
			onActivate: (input) => delegate.activate(input),
			onClose: (input) => delegate.close(input),
		}));
	}

	setEditors(editors: readonly EditorTabDescriptor[], activeInput: EditorInput | undefined): void {
		const activeKey = activeInput ? editorInputKey(activeInput) : undefined;
		this.tabList.setTabs(editors.map((editor) => {
			const label = editorInputLabel(editor.input);
			return {
				id: editorInputKey(editor.input),
				value: editor.input,
				label,
				tooltip: editor.input.resource.toString(),
				preview: editor.preview,
				tabId: editor.tabId,
				panelId: editor.panelId,
			};
		}), activeKey);
		this.tabList.element.hidden = editors.length === 0;
	}
}

function editorInputLabel(input: EditorInput): string {
	if (input.label?.trim()) return input.label;
	const path = decodeURIComponent(input.resource.path).replace(/\/+$/, "");
	const separator = path.lastIndexOf("/");
	return path.slice(separator + 1) || input.resource.toString();
}

import "./media/multiEditorTabsControl.css";
import { DataTransfers } from "../../../../base/browser/dnd.js";
import { TabList } from "../../../../base/browser/ui/tablist/tabList.js";
import { containsExternalEditorDrop } from "./editorDropData.js";
import type { EditorInput } from "./editorInput.js";
import { EditorTabsControl, editorInputKey, type EditorTabDescriptor, type EditorTabsDelegate } from "./editorTabsControl.js";

const DRAG_OVER_ACTIVATE_DELAY = 1500;

/** Renders every open Editor in one reorderable tab list. */
export class MultiEditorTabsControl extends EditorTabsControl {
	private readonly tabList: TabList<EditorTabDescriptor>;
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
				onDragStart: (editor, event) => {
					event.dataTransfer?.setData(DataTransfers.Text, editor.instanceId);
					if (event.dataTransfer) event.dataTransfer.effectAllowed = "move";
					delegate.startDrag(editor.input);
				},
				onDragEnter: (_target, _position, event) => {
					this.previewedInput = undefined;
					if (event.dataTransfer) event.dataTransfer.dropEffect = delegate.isDragging() ? "move" : "copy";
				},
				onDragOver: (target, _position, event, duration) => {
					if (event.dataTransfer) event.dataTransfer.dropEffect = delegate.isDragging() ? "move" : "copy";
					if (target && duration >= DRAG_OVER_ACTIVATE_DELAY && target.input !== this.previewedInput) {
						this.previewedInput = target.input;
						delegate.preview(target.input);
					}
				},
				onDragLeave: () => {
					this.previewedInput = undefined;
				},
				onDrop: (target, position, event) => {
					this.previewedInput = undefined;
					event.stopPropagation();
					if (delegate.isDragging()) delegate.drop(target?.input, position);
					else delegate.dropExternal(event, target?.input, position);
				},
				onDragEnd: () => {
					this.previewedInput = undefined;
					delegate.endDrag();
				},
			},
			onActivate: (editor) => delegate.activate(editor.input),
			onClose: (editor) => delegate.close(editor.input),
		}));
	}

	setEditors(editors: readonly EditorTabDescriptor[], activeInput: EditorInput | undefined): void {
		const activeKey = activeInput ? editors.find(editor => editorInputKey(editor.input) === editorInputKey(activeInput))?.instanceId : undefined;
		this.tabList.setTabs(editors.map((editor) => {
			const label = editorInputLabel(editor.input);
			const state = editor.hasExternalChange ? "conflict" : editor.isDirty ? "dirty" : undefined;
			const stateLabel = editor.hasExternalChange ? "conflict with changes on disk" : editor.isDirty ? "unsaved changes" : undefined;
			return {
				id: editor.instanceId,
				value: editor,
				label,
				tooltip: stateLabel ? `${editor.input.resource.toString()} — ${stateLabel}` : editor.input.resource.toString(),
				ariaLabel: stateLabel ? `${label}, ${stateLabel}` : label,
				...(state ? { state } : {}),
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

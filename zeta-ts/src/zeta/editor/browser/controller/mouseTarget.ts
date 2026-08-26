import { type EditorViewport } from '../view.js';
import { type ClientPoint, type EditorHitTarget, EditorHitTargetKind } from '../../common/viewModel/pointerHitTest.js';

/** Identifies the browser-owned semantic area under one mouse or pointer event. */
export enum MouseTargetKind {
	Text = 'text',
	Gutter = 'gutter',
	LineNumber = 'lineNumber',
	GutterDecoration = 'gutterDecoration',
	EmptyContent = 'emptyContent',
	AfterLines = 'afterLines',
	Widget = 'widget',
	Scrollbar = 'scrollbar',
	ViewZone = 'viewZone',
}

export interface MouseTarget {
	readonly kind: MouseTargetKind;
	readonly editorTarget: EditorHitTarget | undefined;
	readonly element: Element | undefined;
}

/** Resolves DOM event targets into editor semantics without owning gesture state. */
export class MouseTargetFactory {
	constructor(private readonly viewport: EditorViewport) {}

	create(event: Pick<MouseEvent, 'clientX' | 'clientY' | 'target'>, nearest = false): MouseTarget | undefined {
		const element = eventTargetElement(event.target, this.viewport.element.ownerDocument);
		const elementKind = classifyElement(element);
		const editorTarget = nearest
			? this.viewport.getNearestTargetAtClientPoint(event)
			: this.viewport.getTargetAtClientPoint(event);
		if (!editorTarget && elementKind === undefined) return undefined;
		return Object.freeze({
			kind: elementKind ?? kindForEditorTarget(editorTarget!),
			editorTarget,
			element,
		});
	}
}

function eventTargetElement(target: EventTarget | null, ownerDocument: Document): Element | undefined {
	const ElementConstructor = ownerDocument.defaultView?.Element;
	return ElementConstructor && target instanceof ElementConstructor ? target : undefined;
}

function classifyElement(element: Element | undefined): MouseTargetKind | undefined {
	if (!element) return undefined;
	if (element.closest('.stanza-editor-scrollbar-track, .zeta-scrollbar-track, [role="scrollbar"]')) {
		return MouseTargetKind.Scrollbar;
	}
	if (element.closest('.stanza-editor-zone-widget')) return MouseTargetKind.ViewZone;
	if (element.closest('.stanza-editor-widget, .stanza-editor-content-widget, .stanza-editor-overlay-widget')) {
		return MouseTargetKind.Widget;
	}
	if (element.closest('.stanza-editor-line-number')) return MouseTargetKind.LineNumber;
	if (element.closest('.stanza-editor-feature-gutter, .stanza-editor-feature-gutter-slot, .zeta-debug-breakpoint-gutter')) {
		return MouseTargetKind.GutterDecoration;
	}
	return undefined;
}

function kindForEditorTarget(target: EditorHitTarget): MouseTargetKind {
	switch (target.kind) {
		case EditorHitTargetKind.Text:
			return MouseTargetKind.Text;
		case EditorHitTargetKind.Gutter:
			return MouseTargetKind.Gutter;
		case EditorHitTargetKind.EmptyContent:
			return MouseTargetKind.EmptyContent;
		case EditorHitTargetKind.AfterLines:
			return MouseTargetKind.AfterLines;
	}
}

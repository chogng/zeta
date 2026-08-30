import { type EditorViewport } from '../view.js';
import { type ClientPoint, type EditorHitTarget, EditorHitTargetKind } from '../../common/viewModel/pointerHitTest.js';
import { type TextDecorationId } from '../../common/model/decorationCollection.js';
import { GlyphMarginLane } from '../../common/model.js';
import { PartFingerprint, PartFingerprints } from '../view/viewPart.js';

/** Identifies the browser-owned semantic area under one mouse or pointer event. */
export enum SemanticMouseTargetKind {
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

export interface SemanticMouseTarget {
	readonly kind: SemanticMouseTargetKind;
	readonly editorTarget: EditorHitTarget | undefined;
	readonly element: Element | undefined;
	readonly decorationId?: TextDecorationId;
	readonly decorationOwner?: string;
	readonly glyphMarginLane?: GlyphMarginLane;
}

/** Resolves DOM event targets into editor semantics without owning gesture state. */
export class SemanticMouseTargetFactory {
	constructor(private readonly viewport: EditorViewport) {}

	create(event: Pick<MouseEvent, 'clientX' | 'clientY' | 'target'>, nearest = false): SemanticMouseTarget | undefined {
		const element = eventTargetElement(event.target, this.viewport.element.ownerDocument);
		const elementTarget = classifyElement(element, this.viewport.element);
		const editorTarget = nearest
			? this.viewport.getNearestTargetAtClientPoint(event)
			: this.viewport.getTargetAtClientPoint(event);
		if (!editorTarget && elementTarget === undefined) return undefined;
		return Object.freeze({
			kind: elementTarget?.kind ?? kindForEditorTarget(editorTarget!),
			editorTarget,
			element,
			...(elementTarget?.decorationId === undefined ? {} : { decorationId: elementTarget.decorationId }),
			...(elementTarget?.decorationOwner === undefined ? {} : { decorationOwner: elementTarget.decorationOwner }),
			...(elementTarget?.glyphMarginLane === undefined ? {} : { glyphMarginLane: elementTarget.glyphMarginLane }),
		});
	}
}

function eventTargetElement(target: EventTarget | null, ownerDocument: Document): Element | undefined {
	const ElementConstructor = ownerDocument.defaultView?.Element;
	return ElementConstructor && target instanceof ElementConstructor ? target : undefined;
}

interface ElementMouseTarget {
	readonly kind: SemanticMouseTargetKind;
	readonly decorationId?: TextDecorationId;
	readonly decorationOwner?: string;
	readonly glyphMarginLane?: GlyphMarginLane;
}

function classifyElement(element: Element | undefined, editorDomNode: HTMLElement): ElementMouseTarget | undefined {
	if (!element) return undefined;
	if (element.closest('.stanza-editor-scrollbar-track, .zeta-scrollbar-track, [role="scrollbar"]')) {
		return { kind: SemanticMouseTargetKind.Scrollbar };
	}
	if (element.closest('.stanza-editor-zone-widget, .stanza-editor-view-zones, .stanza-editor-margin-view-zones')) return { kind: SemanticMouseTargetKind.ViewZone };
	const fingerprints = PartFingerprints.collect(element, editorDomNode);
	if (element.closest('.stanza-editor-widget, .stanza-editor-content-widget, .stanza-editor-overlay-widget')) return { kind: SemanticMouseTargetKind.Widget };
	if (
		fingerprints.includes(PartFingerprint.ContentWidgets)
		|| fingerprints.includes(PartFingerprint.OverflowingContentWidgets)
		|| fingerprints.includes(PartFingerprint.OverlayWidgets)
		|| fingerprints.includes(PartFingerprint.OverflowingOverlayWidgets)
	) {
		return { kind: SemanticMouseTargetKind.Widget };
	}
	if (element.closest('.line-numbers')) return { kind: SemanticMouseTargetKind.LineNumber };
	const lineDecoration = element.closest<HTMLElement>('.stanza-editor-line-decoration');
	if (lineDecoration) {
		const decorationId = Number(lineDecoration.dataset.decorationId);
		return {
			kind: SemanticMouseTargetKind.GutterDecoration,
			...(Number.isSafeInteger(decorationId) && decorationId > 0 ? { decorationId: decorationId as TextDecorationId } : {}),
			...(lineDecoration.dataset.decorationOwner ? { decorationOwner: lineDecoration.dataset.decorationOwner } : {}),
		};
	}
	const glyph = element.closest<HTMLElement>('.stanza-editor-glyph-margin-decoration');
	const lane = element.closest<HTMLElement>('.stanza-editor-glyph-margin-lane');
	if (glyph || lane) {
		const decorationId = Number(glyph?.dataset.decorationId);
		const glyphMarginLane = readGlyphMarginLane(lane?.dataset.glyphMarginLane);
		return {
			kind: SemanticMouseTargetKind.GutterDecoration,
			...(Number.isSafeInteger(decorationId) && decorationId > 0 ? { decorationId: decorationId as TextDecorationId } : {}),
			...(glyph?.dataset.decorationOwner ? { decorationOwner: glyph.dataset.decorationOwner } : {}),
			...(glyphMarginLane === undefined ? {} : { glyphMarginLane }),
		};
	}
	return undefined;
}

function readGlyphMarginLane(value: string | undefined): GlyphMarginLane | undefined {
	const lane = Number(value);
	return lane === GlyphMarginLane.Left || lane === GlyphMarginLane.Center || lane === GlyphMarginLane.Right ? lane : undefined;
}

function kindForEditorTarget(target: EditorHitTarget): SemanticMouseTargetKind {
	switch (target.kind) {
		case EditorHitTargetKind.Text:
			return SemanticMouseTargetKind.Text;
		case EditorHitTargetKind.Gutter:
			return SemanticMouseTargetKind.Gutter;
		case EditorHitTargetKind.EmptyContent:
			return SemanticMouseTargetKind.EmptyContent;
		case EditorHitTargetKind.AfterLines:
			return SemanticMouseTargetKind.AfterLines;
	}
}

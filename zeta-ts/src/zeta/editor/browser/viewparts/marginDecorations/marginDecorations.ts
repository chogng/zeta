import "./marginDecorations.css";
import { DecorationsOverlay } from "../decorations/decorations.js";
import { type ResolvedDecoration, DecorationPresentation } from '../decorations/decorations.js';
import { type EditorOverlayContext } from '../../view/renderingContext.js';
import { DynamicViewOverlay } from "../../view/dynamicViewOverlay.js";
import { type EditorRenderingContext, EditorViewContext } from "../../view/viewPart.js";
import { ViewPartRows } from '../../view/viewLayer.js';

/** Projects line-level diagnostics into the editor margin. */
export class MarginViewLineDecorationsOverlay extends DynamicViewOverlay {
	public readonly domNode: HTMLElement;
	private readonly decorations: DecorationsOverlay;
	private readonly rows: ViewPartRows;

	constructor(context: EditorViewContext, host: HTMLElement, decorations: DecorationsOverlay) {
		super(context);
		this.rows = this._register(new ViewPartRows(host, 'stanza-editor-margin-decorations-layer', 'stanza-editor-diagnostic-marker'));
		this.domNode = this.rows.domNode;
		this.decorations = decorations;
	}

	public render(context: EditorRenderingContext): void {
		const overlay = context.overlay;
		if (!overlay) {
			return;
		}
		projectStanzaDiagnosticMarginDecorations(overlay, this.decorations.visibleDecorations(overlay), this.rows.render(context));
	}
}

const DIAGNOSTIC_PRESENTATION_PRIORITY = new Map<ResolvedDecoration['presentation'], number>([
	[DecorationPresentation.ErrorUnderline, 4],
	[DecorationPresentation.WarningUnderline, 3],
	[DecorationPresentation.InformationUnderline, 2],
	[DecorationPresentation.HintUnderline, 1],
]);

function projectStanzaDiagnosticMarginDecorations(
	context: EditorOverlayContext,
	decorations: readonly ResolvedDecoration[],
	rows: ReadonlyMap<number, HTMLElement>,
): void {
	const diagnosticsByLine = new Map<number, ResolvedDecoration[]>();
	for (const decoration of decorations) {
		if (!DIAGNOSTIC_PRESENTATION_PRIORITY.has(decoration.presentation)) continue;
		const startLineIndex = decoration.range.startLineNumber - 1;
		const endLineIndex = decoration.range.endColumn === 1 && decoration.range.endLineNumber - 1 > startLineIndex
			? decoration.range.endLineNumber - 2
			: decoration.range.endLineNumber - 1;
		for (let lineIndex = startLineIndex; lineIndex <= endLineIndex; lineIndex += 1) {
			const lineDiagnostics = diagnosticsByLine.get(lineIndex) ?? [];
			lineDiagnostics.push(decoration);
			diagnosticsByLine.set(lineIndex, lineDiagnostics);
		}
	}
	for (const [visualLineIndex, marker] of rows) {
		const visualLine = context.visualLineProjection.lineAt(visualLineIndex);
		const diagnostics = visualLine?.firstForLogicalLine ? diagnosticsByLine.get(visualLine.logicalLineIndex) ?? [] : [];
		marker.hidden = diagnostics.length === 0;
		delete marker.dataset.diagnosticHoverText;
		marker.removeAttribute('title');
		if (diagnostics.length === 0) {
			marker.className = 'stanza-editor-diagnostic-marker';
			marker.textContent = '';
			continue;
		}
		const highest = diagnostics.reduce((current, candidate) =>
			(DIAGNOSTIC_PRESENTATION_PRIORITY.get(candidate.presentation) ?? 0) > (DIAGNOSTIC_PRESENTATION_PRIORITY.get(current.presentation) ?? 0) ? candidate : current);
		marker.className = `stanza-editor-diagnostic-marker ${diagnosticMarkerClass(highest.presentation)}`;
		marker.textContent = '●';
		const hoverTexts = [...new Set(diagnostics.flatMap(diagnostic => diagnostic.hoverText === undefined ? [] : [diagnostic.hoverText]))];
		if (hoverTexts.length > 0) {
			const hoverText = hoverTexts.join('\n');
			marker.dataset.diagnosticHoverText = hoverText;
			marker.title = hoverText;
		}
	}
}

function diagnosticMarkerClass(presentation: ResolvedDecoration['presentation']): 'error' | 'warning' | 'information' | 'hint' {
	switch (presentation) {
		case DecorationPresentation.ErrorUnderline: return 'error';
		case DecorationPresentation.WarningUnderline: return 'warning';
		case DecorationPresentation.InformationUnderline: return 'information';
		case DecorationPresentation.HintUnderline: return 'hint';
		default: throw new TypeError(`Unknown diagnostic presentation '${presentation}'`);
	}
}

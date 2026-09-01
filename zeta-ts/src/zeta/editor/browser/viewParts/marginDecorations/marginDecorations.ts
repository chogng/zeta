import "./marginDecorations.css";
import { DecorationsOverlay } from "../decorations/decorations.js";
import { type ResolvedDecoration, DecorationPresentation } from '../decorations/decorations.js';
import { type RenderingContext } from '../../view/renderingContext.js';
import { DynamicViewOverlay } from '../../view/dynamicViewOverlay.js';
import { type ViewContext } from '../../../common/viewModel/viewContext.js';
import { type EditorVisualLineProjection } from '../../../common/viewModel/modelLineProjection.js';
import * as viewEvents from '../../../common/viewEvents.js';
import { renderViewPartRows } from '../../view/viewLayer.js';

/** Projects line-level diagnostics into the editor margin. */
export class MarginViewLineDecorationsOverlay extends DynamicViewOverlay {
	private _renderResult: string[] = [];
	constructor(private readonly _context: ViewContext, private readonly decorations: DecorationsOverlay, private readonly ownerDocument: Document, private readonly readVisualProjection: () => EditorVisualLineProjection) {
		super();
		this._context.addEventHandler(this);
		this._register(this.decorations.onDidChange(() => this.forceShouldRender()));
	}

	public override dispose(): void {
		this._context.removeEventHandler(this);
		super.dispose();
	}

	public override onConfigurationChanged(_event: viewEvents.ViewConfigurationChangedEvent): boolean { return true; }
	public override onDecorationsChanged(_event: viewEvents.ViewDecorationsChangedEvent): boolean { return true; }
	public override onFlushed(_event: viewEvents.ViewFlushedEvent): boolean { return true; }
	public override onLinesChanged(_event: viewEvents.ViewLinesChangedEvent): boolean { return true; }
	public override onLinesDeleted(_event: viewEvents.ViewLinesDeletedEvent): boolean { return true; }
	public override onLinesInserted(_event: viewEvents.ViewLinesInsertedEvent): boolean { return true; }
	public override onScrollChanged(_event: viewEvents.ViewScrollChangedEvent): boolean { return true; }
	public override onZonesChanged(_event: viewEvents.ViewZonesChangedEvent): boolean { return true; }

	public prepareRender(context: RenderingContext): void {
		this._renderResult = renderViewPartRows(context, this.ownerDocument, rows => {
			projectStanzaDiagnosticMarginDecorations(this.readVisualProjection(), this._getDecorations(context), rows);
		});
	}

	public render(startLineNumber: number, lineNumber: number): string {
		return this._renderResult[lineNumber - startLineNumber] ?? '';
	}

	private _getDecorations(context: RenderingContext): readonly ResolvedDecoration[] {
		return this.decorations.visibleDecorations(context);
	}
}

const DIAGNOSTIC_PRESENTATION_PRIORITY = new Map<ResolvedDecoration['presentation'], number>([
	[DecorationPresentation.ErrorUnderline, 4],
	[DecorationPresentation.WarningUnderline, 3],
	[DecorationPresentation.InformationUnderline, 2],
	[DecorationPresentation.HintUnderline, 1],
]);

function projectStanzaDiagnosticMarginDecorations(
	projection: EditorVisualLineProjection,
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
	for (const [visualLineIndex, row] of rows) {
		const marker = h(row.ownerDocument, 'div');
		row.append(marker);
		const visualLine = projection.lineAt(visualLineIndex);
		const diagnostics = visualLine?.firstForLogicalLine ? diagnosticsByLine.get(visualLine.logicalLineIndex) ?? [] : [];
		marker.hidden = diagnostics.length === 0;
		delete marker.dataset.diagnosticHoverText;
		marker.removeAttribute('title');
		if (diagnostics.length === 0) {
			marker.className = 'cmdr stanza-editor-diagnostic-marker';
			marker.textContent = '';
			continue;
		}
		const highest = diagnostics.reduce((current, candidate) =>
			(DIAGNOSTIC_PRESENTATION_PRIORITY.get(candidate.presentation) ?? 0) > (DIAGNOSTIC_PRESENTATION_PRIORITY.get(current.presentation) ?? 0) ? candidate : current);
		marker.className = `cmdr stanza-editor-diagnostic-marker ${diagnosticMarkerClass(highest.presentation)}`;
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
import { h } from '../../../../base/browser/dom.js';

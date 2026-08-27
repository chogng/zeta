import { type Event } from '../../../../base/common/event.js';
import { Disposable } from '../../../../base/common/lifecycle.js';
import { type URI } from '../../../../base/common/uri.js';
import { createStanzaDecorationSource, DecorationPresentation, type DecorationSource, type OwnedDecorationSource, type ResolvedDecoration } from '../../../../editor/browser/viewparts/decorations/decorationPresentation.js';
import { TextPosition, TextRange } from '../../../../editor/common/core/text.js';
import { LineDiffKind, type LineDiffRow } from '../../../../editor/common/diff/lineDiff.js';
import { TextDecorationCollection } from '../../../../editor/common/model/decorationCollection.js';
import { type TextModel } from '../../../../editor/common/model/textModel.js';
import { TrackedRangeStickiness } from '../../../../editor/common/model/trackedRange.js';
import { type IConfigurationService } from '../../../../platform/configuration/common/configurationService.js';
import { type IDiffApi } from '../../../../platform/diff/common/diffApi.js';
import { type IQuickDiffModelService, type QuickDiffComparison } from '../common/quickDiff.js';
import { ScmConfiguration } from '../common/scmConfiguration.js';

interface QuickDiffDecorationMetadata {
	readonly presentation: DecorationPresentation.DiffAdded | DecorationPresentation.DiffModified | DecorationPresentation.DiffDeleted;
	readonly providerLabels: readonly string[];
}

/** Projects one shared Quick Diff model into gutter, overview-ruler, and minimap decorations. */
export class QuickDiffDecorator extends Disposable implements OwnedDecorationSource {
	private readonly collection: TextDecorationCollection<QuickDiffDecorationMetadata>;
	private readonly source: DecorationSource;
	readonly onDidChange: Event<void>;

	constructor(resource: URI, private readonly model: TextModel, diffApi: IDiffApi, modelService: IQuickDiffModelService, private readonly configurationService: IConfigurationService) {
		super();
		const modelReference = this._register(modelService.createModelReference(resource, model, diffApi));
		this.collection = this._register(new TextDecorationCollection<QuickDiffDecorationMetadata>(model));
		this.source = createStanzaDecorationSource(this.collection, decoration => this.resolve(decoration.metadata), decoration => hoverText(decoration.metadata));
		this.onDidChange = this.source.onDidChange;
		this._register(modelReference.object.onDidChange(() => this.rebuild(modelReference.object.state.comparisons)));
		this._register(configurationService.onDidChangeConfiguration(event => {
			if (event.affectsConfiguration(ScmConfiguration.diffDecorations)) this.rebuild(modelReference.object.state.comparisons);
		}));
		this.rebuild(modelReference.object.state.comparisons);
	}

	get decorations(): readonly ResolvedDecoration[] {
		return this.source.decorations;
	}

	private resolve(metadata: QuickDiffDecorationMetadata) {
		const setting = this.configurationService.getValue(ScmConfiguration.diffDecorations);
		if (setting === 'none') return undefined;
		const gutter = setting === 'all' || setting === 'gutter';
		return Object.freeze({
			presentation: metadata.presentation,
			...(gutter ? { linesDecoration: { className: `zeta-quick-diff-gutter ${classNameForPresentation(metadata.presentation)}`, tooltip: hoverText(metadata) } } : {}),
			overviewRuler: setting === 'all' || setting === 'overview',
			minimap: setting === 'all' || setting === 'minimap',
		});
	}

	private rebuild(comparisons: readonly QuickDiffComparison[]): void {
		const setting = this.configurationService.getValue(ScmConfiguration.diffDecorations);
		if (setting === 'none') {
			this.collection.clear();
			return;
		}
		const byLine = new Map<number, QuickDiffDecorationMetadata>();
		for (const comparison of comparisons) {
			const rows = comparison.model.diff?.rows ?? [];
			for (let rowIndex = 0; rowIndex < rows.length; rowIndex += 1) {
				const row = rows[rowIndex]!;
				if (row.kind === LineDiffKind.Unchanged) continue;
				const lineIndex = row.modifiedLineIndex ?? deletionAnchor(rows, rowIndex, this.model.lineCount);
				const presentation = presentationForRow(row);
				const current = byLine.get(lineIndex);
				if (!current || presentationPriority(presentation) > presentationPriority(current.presentation)) {
					byLine.set(lineIndex, Object.freeze({ presentation, providerLabels: Object.freeze([comparison.original.label]) }));
				} else if (!current.providerLabels.includes(comparison.original.label)) {
					byLine.set(lineIndex, Object.freeze({ ...current, providerLabels: Object.freeze([...current.providerLabels, comparison.original.label]) }));
				}
			}
		}
		this.collection.replaceAll(Object.freeze([...byLine.entries()].sort(([left], [right]) => left - right).map(([lineIndex, metadata]) => Object.freeze({
			range: TextRange.from(TextPosition.at(lineIndex, 0), TextPosition.at(lineIndex, this.model.getLineLength(lineIndex))),
			stickiness: TrackedRangeStickiness.NeverGrowsAtEdges,
			metadata,
		}))));
	}
}

function deletionAnchor(rows: readonly LineDiffRow[], rowIndex: number, lineCount: number): number {
	for (let index = rowIndex + 1; index < rows.length; index += 1) {
		const lineIndex = rows[index]?.modifiedLineIndex;
		if (lineIndex !== undefined) return lineIndex;
	}
	for (let index = rowIndex - 1; index >= 0; index -= 1) {
		const lineIndex = rows[index]?.modifiedLineIndex;
		if (lineIndex !== undefined) return lineIndex;
	}
	return Math.max(0, lineCount - 1);
}

function presentationForRow(row: LineDiffRow): QuickDiffDecorationMetadata['presentation'] {
	switch (row.kind) {
		case LineDiffKind.Added: return DecorationPresentation.DiffAdded;
		case LineDiffKind.Modified: return DecorationPresentation.DiffModified;
		case LineDiffKind.Removed: return DecorationPresentation.DiffDeleted;
		case LineDiffKind.Unchanged: throw new TypeError('Unchanged rows do not create Quick Diff decorations');
	}
}

function presentationPriority(presentation: DecorationPresentation): number {
	switch (presentation) {
		case DecorationPresentation.DiffDeleted: return 3;
		case DecorationPresentation.DiffModified: return 2;
		case DecorationPresentation.DiffAdded: return 1;
		default: return 0;
	}
}

function classNameForPresentation(presentation: QuickDiffDecorationMetadata['presentation']): string {
	switch (presentation) {
		case DecorationPresentation.DiffAdded: return 'zeta-quick-diff-added';
		case DecorationPresentation.DiffModified: return 'zeta-quick-diff-modified';
		case DecorationPresentation.DiffDeleted: return 'zeta-quick-diff-deleted';
	}
}

function hoverText(metadata: QuickDiffDecorationMetadata): string {
	const kind = metadata.presentation === DecorationPresentation.DiffAdded
		? 'Added'
		: metadata.presentation === DecorationPresentation.DiffModified ? 'Modified' : 'Deleted';
	return `${kind} relative to ${metadata.providerLabels.join(', ')}`;
}

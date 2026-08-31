import { addDisposableListener, stopEvent } from '../../../../base/browser/dom.js';
import { CancellationTokenSource } from '../../../../base/common/cancellation.js';
import { Disposable, DisposableStore, toDisposable } from '../../../../base/common/lifecycle.js';
import { registerTextEditorCapabilityContribution } from '../../../browser/editorExtensions.js';
import { type ICodeEditor } from '../../../browser/editorBrowser.js';
import { type View } from '../../../browser/view.js';
import { type ViewController } from '../../../browser/view/viewController.js';
import { extendEditorEditCommand } from '../../../common/commands/editorCommand.js';
import { type EditorEditCommand } from '../../../common/commands/editorEditCommand.js';
import { type CursorsController } from '../../../common/cursor/cursor.js';
import { Position } from '../../../common/core/position.js';
import { Range } from '../../../common/core/range.js';
import { type LanguageFeatureRegistry } from '../../../common/languageFeatureRegistry.js';
import { type LinkedEditingRangeProvider, type LinkedEditingRanges } from '../../../common/languages.js';
import { TrackedRangeStickiness } from '../../../common/model.js';
import { type TrackedRange } from '../../../common/model/trackedRange.js';
import './linkedEditing.css';

export class LinkedEditingContribution extends Disposable {
	public static readonly ID = 'editor.contrib.linkedEditing';

	public static get(editor: ICodeEditor): LinkedEditingContribution | null {
		return editor.getContribution<LinkedEditingContribution>(LinkedEditingContribution.ID);
	}

	private readonly ranges = this._register(new DisposableStore());
	private trackedRanges: readonly TrackedRange[] = [];
	private isActive = false;
	private isActivationScheduled = false;
	private request: CancellationTokenSource | undefined;
	private wordPattern: RegExp | undefined;

	constructor(
		private readonly view: ViewController,
		input: HTMLElement,
		private readonly viewport: View,
		private readonly selections: CursorsController,
		private readonly providers: LanguageFeatureRegistry<LinkedEditingRangeProvider>,
		private readonly defaultWordPattern: () => RegExp | undefined,
		private readonly onError: (error: unknown) => void = error => console.error('Stanza linked editing failed', error),
	) {
		super();
		if (viewport.textModel !== selections.textModel) throw new TypeError('Linked editing dependencies must share a text model');
		this._register(addDisposableListener(input, 'keydown', event => {
			if (event.defaultPrevented || event.isComposing || !event.shiftKey || (!event.ctrlKey && !event.metaKey) || event.altKey || event.key.toLowerCase() !== 'l') return;
			stopEvent(event);
			void this.activate();
		}, true));
		this._register(addDisposableListener(input, 'keydown', event => {
			if (event.key !== 'Escape' || !this.isActive) return;
			stopEvent(event);
			this.clear();
		}, true));
		this._register(view.registerCommandTransformer(command => this.extendCommand(command)));
		this._register(selections.onDidChange(() => this.scheduleActivation()));
		this._register(providers.onDidChange(() => this.scheduleActivation()));
		this._register(toDisposable(() => this.clear()));
		this.scheduleActivation();
	}

	private async activate(): Promise<void> {
		if (this.isDisposed) return;
		this.request?.dispose(true);
		const request = this.request = new CancellationTokenSource();
		try {
			const primary = this.selections.selections[0]!;
			if (!primary.isEmpty()) {
				this.clear();
				return;
			}

			const model = this.viewport.textModel;
			const version = model.getVersionId();
			let result: LinkedEditingRanges | undefined;
			for (const provider of this.providers.ordered(model)) {
				try {
					result = await Promise.resolve(provider.provideLinkedEditingRanges(model, primary.getPosition(), request.token)) ?? undefined;
				} catch (error) {
					if (request.token.isCancellationRequested) return;
					this.onError(error);
					continue;
				}
				if (request.token.isCancellationRequested || this.isDisposed || model.getVersionId() !== version) return;
				if (result) break;
			}

			if (!result || result.ranges.length < 2) {
				this.clear();
				return;
			}
			const normalizedRanges = result.ranges.map(range => model.validateRange(range));
			const expectedText = model.getTextInRange(normalizedRanges[0]!);
			if (normalizedRanges.some(range => model.getTextInRange(range) !== expectedText)) {
				this.clear();
				return;
			}

			this.ranges.clear();
			this.trackedRanges = normalizedRanges.map(range => {
				const trackedRange = model.trackRange(range, TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges);
				this.ranges.add(toDisposable(() => trackedRange.dispose()));
				return trackedRange;
			});
			this.wordPattern = result.wordPattern ?? this.defaultWordPattern();
			this.isActive = true;
			this.viewport.element.classList.add('linked-editing-active');
			this.viewport.announceAccessibilityStatus(`${normalizedRanges.length} linked editing ranges active`);
		} finally {
			if (this.request === request) {
				this.request = undefined;
				request.dispose();
			}
		}
	}

	private extendCommand(command: EditorEditCommand): EditorEditCommand {
		if (!this.isActive || command.edits.length !== 1) return command;
		const sourceEdit = command.edits[0]!;
		const sourceRange = Range.lift(sourceEdit.range);
		const source = this.trackedRanges.find(candidate => candidate.range.containsRange(sourceRange));
		if (!source) return command;

		const model = this.viewport.textModel;
		const sourceStart = model.offsetAt(source.range.getStartPosition());
		const relativeStart = model.offsetAt(sourceRange.getStartPosition()) - sourceStart;
		const relativeEnd = model.offsetAt(sourceRange.getEndPosition()) - sourceStart;
		const currentValue = model.getTextInRange(source.range);
		const nextValue = currentValue.slice(0, relativeStart) + sourceEdit.text + currentValue.slice(relativeEnd);
		if (this.wordPattern && !matchesEntirePattern(this.wordPattern, nextValue)) {
			this.clear();
			return command;
		}

		const edits = this.trackedRanges
			.filter(candidate => candidate !== source)
			.map(candidate => {
				const targetStart = model.offsetAt(candidate.range.getStartPosition());
				const targetEnd = model.offsetAt(candidate.range.getEndPosition());
				const start = targetStart + relativeStart;
				const end = targetStart + relativeEnd;
				if (start < targetStart || end > targetEnd) return undefined;
				return { range: Range.fromPositions(model.positionAt(start), model.positionAt(end)), text: sourceEdit.text };
			})
			.filter((edit): edit is { readonly range: Range; readonly text: string } => edit !== undefined)
			.sort((left, right) => Position.compare(left.range.getStartPosition(), right.range.getStartPosition()));
		return extendEditorEditCommand(model, command, edits);
	}

	private scheduleActivation(): void {
		if (this.isActivationScheduled || this.isDisposed) return;
		this.isActivationScheduled = true;
		queueMicrotask(() => {
			this.isActivationScheduled = false;
			if (!this.isDisposed) void this.activate();
		});
	}

	private clear(): void {
		this.request?.dispose(true);
		this.request = undefined;
		this.ranges.clear();
		this.trackedRanges = [];
		this.wordPattern = undefined;
		this.isActive = false;
		this.viewport.element.classList.remove('linked-editing-active');
	}
}

function matchesEntirePattern(pattern: RegExp, value: string): boolean {
	pattern.lastIndex = 0;
	const match = pattern.exec(value);
	pattern.lastIndex = 0;
	return match?.index === 0 && match[0].length === value.length;
}

registerTextEditorCapabilityContribution({
	id: LinkedEditingContribution.ID,
	install: context => {
		if (context.kind !== 'text') return;
		context.register(new LinkedEditingContribution(
			context.view,
			context.view.element,
			context.viewport,
			context.viewModel,
			context.languageFeaturesService.linkedEditingRangeProvider,
			() => context.configurations.getLanguageConfiguration(context.languageId).getWordDefinition(),
			context.onLanguageError,
		));
	},
});

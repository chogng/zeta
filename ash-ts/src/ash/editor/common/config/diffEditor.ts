import type { IDiffEditorBaseOptions, ValidDiffEditorBaseOptions } from './editorOptions.js';

/** Default options shared by side-by-side and inline diff hosts. */
export const diffEditorDefaultOptions = Object.freeze({
	enableSplitViewResizing: true,
	splitViewDefaultRatio: 0.5,
	renderSideBySide: true,
	renderMarginRevertIcon: true,
	renderGutterMenu: true,
	maxComputationTime: 5_000,
	maxFileSize: 50,
	ignoreTrimWhitespace: true,
	renderIndicators: true,
	originalEditable: false,
	diffCodeLens: false,
	renderOverviewRuler: true,
	diffWordWrap: 'inherit' as const,
	diffAlgorithm: 'advanced' as const,
	accessibilityVerbose: false,
	experimental: Object.freeze({
		showMoves: false,
		showEmptyDecorations: true,
		useTrueInlineView: false,
	}),
	hideUnchangedRegions: Object.freeze({
		enabled: false,
		contextLineCount: 3,
		minimumLineCount: 3,
		revealLineCount: 20,
	}),
	isInEmbeddedEditor: false,
	onlyShowAccessibleDiffViewer: false,
	renderSideBySideInlineBreakpoint: 900,
	useInlineViewWhenSpaceIsLimited: true,
	compactMode: false,
	hideOriginalLineNumbers: false,
}) satisfies ValidDiffEditorBaseOptions;

/** Returns a validated copy of the diff options with common defaults applied. */
export function resolveDiffEditorOptions(options: IDiffEditorBaseOptions = {}): ValidDiffEditorBaseOptions {
	if (!options || typeof options !== 'object') throw new TypeError('Diff editor options must be an object');
	const resolved = {
		...diffEditorDefaultOptions,
		...options,
		experimental: { ...diffEditorDefaultOptions.experimental, ...options.experimental },
		hideUnchangedRegions: { ...diffEditorDefaultOptions.hideUnchangedRegions, ...options.hideUnchangedRegions },
	};
	if (!Number.isFinite(resolved.splitViewDefaultRatio) || resolved.splitViewDefaultRatio < 0 || resolved.splitViewDefaultRatio > 1) {
		throw new RangeError('Diff editor split view ratio must be between 0 and 1');
	}
	if (!Number.isSafeInteger(resolved.maxComputationTime) || resolved.maxComputationTime < 0) {
		throw new RangeError('Diff editor maximum computation time must be a non-negative integer');
	}
	if (!Number.isFinite(resolved.maxFileSize) || resolved.maxFileSize < 0) {
		throw new RangeError('Diff editor maximum file size must be non-negative');
	}
	if (!Number.isSafeInteger(resolved.renderSideBySideInlineBreakpoint) || resolved.renderSideBySideInlineBreakpoint < 0) {
		throw new RangeError('Diff editor inline breakpoint must be a non-negative integer');
	}
	return Object.freeze({
		...resolved,
		experimental: Object.freeze(resolved.experimental),
		hideUnchangedRegions: Object.freeze(resolved.hideUnchangedRegions),
	});
}

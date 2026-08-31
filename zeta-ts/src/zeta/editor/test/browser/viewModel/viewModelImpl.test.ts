import assert from 'node:assert/strict';
import test from 'node:test';
import { JSDOM } from 'jsdom';
import { Disposable, toDisposable } from '../../../../base/common/lifecycle.js';
import { ThemeService } from '../../../../platform/theme/common/themeService.js';
import { darkColorTheme } from '../../../../platform/theme/common/colorTheme.js';
import { EditorConfiguration } from '../../../browser/config/editorConfiguration.js';
import { CursorState } from '../../../common/cursorCommon.js';
import { CursorChangeReason } from '../../../common/cursorEvents.js';
import { Position } from '../../../common/core/position.js';
import { Range } from '../../../common/core/range.js';
import { Selection } from '../../../common/core/selection.js';
import { ScrollType } from '../../../common/editorCommon.js';
import { createBuiltinLanguageConfigurationService } from '../../../common/languages/languageBuiltinConfigurations.js';
import { TextModel } from '../../../common/model/textModel.js';
import { MonospaceLineBreaksComputerFactory } from '../../../common/viewModel/monospaceLineBreaksComputer.js';
import { ViewModel } from '../../../common/viewModel/viewModelImpl.js';

test('ViewModel owns line projection, cursor, layout, and visible-line publication', () => {
	const dom = new JSDOM('<!doctype html><body><main></main></body>');
	const container = dom.window.document.querySelector('main')!;
	using cleanup = toDisposable(() => dom.window.close());
	using configuration = new EditorConfiguration({
		dimension: { width: 200, height: 40 },
		lineHeight: 20,
	}, container);
	using model = new TextModel('one\ntwo\nthree');
	using languages = createBuiltinLanguageConfigurationService();
	using theme = new ThemeService(darkColorTheme);
	const factory = MonospaceLineBreaksComputerFactory.create(configuration.options);
	const visible: Array<{ startLineNumber: number; endLineNumber: number }> = [];
	using viewModel = new ViewModel(
		1,
		configuration,
		model,
		factory,
		factory,
		callback => {
			queueMicrotask(callback);
			return Disposable.None;
		},
		languages,
		theme,
		{ setVisibleLines: ranges => visible.push(...ranges) },
		{ batchChanges: callback => callback() },
	);

	assert.equal(viewModel.getLineCount(), 3);
	assert.equal(viewModel.viewLayout.getScrollHeight(), 60);
	assert.deepEqual(viewModel.getPrimaryCursorState().modelState.position, new Position(1, 1));

	viewModel.setCursorStates('test', CursorChangeReason.Explicit, [CursorState.fromModelSelection(new Selection(3, 2, 3, 2))]);
	assert.deepEqual(viewModel.getPrimaryCursorState().modelState.position, new Position(3, 2));
	const cursorState = viewModel.saveCursorState();
	viewModel.setSelections('test', [new Selection(1, 2, 1, 2)], CursorChangeReason.Explicit);
	viewModel.restoreCursorState(cursorState);
	assert.deepEqual(viewModel.getSelection(), new Selection(3, 2, 3, 2));
	assert.deepEqual(viewModel.getSelections(), [new Selection(3, 2, 3, 2)]);

	viewModel.viewLayout.setScrollPosition({ scrollTop: 20 }, ScrollType.Immediate);
	const viewState = viewModel.saveState();
	assert.deepEqual(viewModel.reduceRestoreState(viewState), { scrollLeft: 0, scrollTop: 20 });

	viewModel.setHiddenAreas([new Range(2, 1, 2, 1)]);
	assert.equal(viewModel.getLineCount(), 2);
	assert.equal(viewModel.getLineContent(2), 'three');
	assert.deepEqual(viewModel.getVisibleRanges(), [new Range(1, 1, 1, 4), new Range(3, 1, 3, 6)]);
	model.applyEdits([{
		range: Range.fromPositions(new Position(1, 4)),
		text: '!',
	}]);
	assert.equal(viewModel.getLineContent(1), 'one!');
	viewModel.visibleLinesStabilized();
	assert.deepEqual(visible.at(-1), { startLineNumber: 1, endLineNumber: 3 });
});

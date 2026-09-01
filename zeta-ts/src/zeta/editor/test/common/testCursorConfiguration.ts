import { Event } from '../../../base/common/event.js';
import { Disposable } from '../../../base/common/lifecycle.js';
import { type IEditorConfiguration } from '../../common/config/editorConfiguration.js';
import { EditorOption, editorOptionsRegistry, type ConfigurationChangedEvent, type FindComputedEditorOptionValueById, type IComputedEditorOptions, type IEditorOptions } from '../../common/config/editorOptions.js';
import { FontInfo } from '../../common/config/fontInfo.js';
import { CursorConfiguration } from '../../common/cursorCommon.js';
import { CursorsController } from '../../common/cursor/cursor.js';
import { IdentityCoordinatesConverter } from '../../common/coordinatesConverter.js';
import { type Selection } from '../../common/core/selection.js';
import { createBuiltinLanguageConfigurationService } from '../../common/languages/languageBuiltinConfigurations.js';
import { type ILanguageConfigurationService } from '../../common/languages/languageConfigurationRegistry.js';
import { type TextModel } from '../../common/model/textModel.js';

export interface TestCursorsControllerOptions extends IEditorOptions {
	readonly selectionHistoryLimit?: number;
	readonly cursorHistoryLimit?: number;
}

export function createTestCursorsController(
	model: TextModel,
	selections: readonly Selection[],
	options: TestCursorsControllerOptions = {},
): CursorsController {
	const languageConfigurationService = createBuiltinLanguageConfigurationService();
	const cursorConfig = createTestCursorConfiguration(model, languageConfigurationService, options);
	const controller = new TestCursorsController(
		model,
		new IdentityCoordinatesConverter(model),
		cursorConfig,
		languageConfigurationService,
		options,
	);
	controller.setSelections(selections);
	return controller;
}

class TestCursorsController extends CursorsController {
	constructor(
		model: TextModel,
		coordinatesConverter: IdentityCoordinatesConverter,
		cursorConfig: CursorConfiguration,
		private readonly languageConfigurationService: ReturnType<typeof createBuiltinLanguageConfigurationService>,
		options: TestCursorsControllerOptions,
	) {
		super(model, model, coordinatesConverter, cursorConfig, options);
	}

	public override dispose(): void {
		super.dispose();
		this.languageConfigurationService.dispose();
	}
}

export function createTestCursorConfiguration(model: TextModel, languageConfigurationService: ILanguageConfigurationService, rawOptions: IEditorOptions = {}): CursorConfiguration {
	const fontInfo = new FontInfo({
		pixelRatio: 1,
		fontFamily: 'monospace',
		fontWeight: 'normal',
		fontSize: 10,
		fontFeatureSettings: 'none',
		fontVariationSettings: 'normal',
		lineHeight: 20,
		letterSpacing: 0,
		isMonospace: true,
		typicalHalfwidthCharacterWidth: 10,
		typicalFullwidthCharacterWidth: 20,
		canUseHalfwidthRightwardsArrow: true,
		spaceWidth: 10,
		middotWidth: 10,
		wsmiddotWidth: 10,
		maxDigitWidth: 10,
	}, true);
	const options: IComputedEditorOptions = {
		get<T extends EditorOption>(id: T): FindComputedEditorOptionValueById<T> {
			if (id === EditorOption.fontInfo) return fontInfo as unknown as FindComputedEditorOptionValueById<T>;
			if (id === EditorOption.layoutInfo) return { height: 200 } as unknown as FindComputedEditorOptionValueById<T>;
			const option = editorOptionsRegistry[id];
			if (!option) throw new ReferenceError(`Missing editor option ${id}`);
			return option.validate((rawOptions as Record<string, unknown>)[option.name]) as FindComputedEditorOptionValueById<T>;
		},
	};
	const configuration: IEditorConfiguration = {
		isSimpleWidget: false,
		contextMenuId: undefined,
		options,
		onDidChangeFast: Event.None as Event<ConfigurationChangedEvent>,
		onDidChange: Event.None as Event<ConfigurationChangedEvent>,
		getRawOptions: () => ({ ...rawOptions }),
		updateOptions: () => {},
		observeContainer: () => {},
		setIsDominatedByLongLines: () => {},
		setModelLineCount: () => {},
		setViewLineCount: () => {},
		setReservedHeight: () => {},
		setGlyphMarginDecorationLaneCount: () => {},
		dispose: Disposable.None.dispose,
		[Symbol.dispose]: Disposable.None[Symbol.dispose],
	};
	return new CursorConfiguration(model.getLanguageId(), model.getOptions(), configuration, languageConfigurationService);
}

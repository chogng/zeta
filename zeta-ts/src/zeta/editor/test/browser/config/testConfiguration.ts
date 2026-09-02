import { EditorConfiguration } from '../../../browser/config/editorConfiguration.js';
import { EditorFontLigatures, EditorFontVariations, type IEditorOptions } from '../../../common/config/editorOptions.js';
import { type BareFontInfo, FontInfo } from '../../../common/config/fontInfo.js';
import { MenuId } from '../../../../platform/actions/common/actions.js';

export function createTestConfiguration(container: HTMLElement, options: IEditorOptions = {}): EditorConfiguration {
	return new TestEditorConfiguration(false, MenuId.EditorContext, options, container);
}

class TestEditorConfiguration extends EditorConfiguration {
	protected override _readFontInfo(font: BareFontInfo): FontInfo {
		return new FontInfo({
			...font,
			isMonospace: TEST_FONT_INFO.isMonospace,
			typicalHalfwidthCharacterWidth: TEST_FONT_INFO.typicalHalfwidthCharacterWidth,
			typicalFullwidthCharacterWidth: TEST_FONT_INFO.typicalFullwidthCharacterWidth,
			canUseHalfwidthRightwardsArrow: TEST_FONT_INFO.canUseHalfwidthRightwardsArrow,
			spaceWidth: TEST_FONT_INFO.spaceWidth,
			middotWidth: TEST_FONT_INFO.middotWidth,
			wsmiddotWidth: TEST_FONT_INFO.wsmiddotWidth,
			maxDigitWidth: TEST_FONT_INFO.maxDigitWidth,
		}, true);
	}
}

export const TEST_FONT_INFO = new FontInfo({
	pixelRatio: 1,
	fontFamily: 'Zeta Test Mono',
	fontWeight: 'normal',
	fontSize: 14,
	fontFeatureSettings: EditorFontLigatures.OFF,
	fontVariationSettings: EditorFontVariations.OFF,
	lineHeight: 20,
	letterSpacing: 0,
	isMonospace: true,
	typicalHalfwidthCharacterWidth: 8,
	typicalFullwidthCharacterWidth: 16,
	canUseHalfwidthRightwardsArrow: true,
	spaceWidth: 8,
	middotWidth: 8,
	wsmiddotWidth: 8,
	maxDigitWidth: 8,
}, true);

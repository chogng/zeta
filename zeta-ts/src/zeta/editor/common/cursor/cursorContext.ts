import { type ICoordinatesConverter } from '../coordinatesConverter.js';
import { type CursorConfiguration, type ICursorSimpleModel } from '../cursorCommon.js';
import { type ITextModel } from '../model.js';

/** Shared model/view/configuration state consumed by all cursor operations. */
export class CursorContext {
	_cursorContextBrand: void = undefined;

	constructor(
		public readonly model: ITextModel,
		public readonly viewModel: ICursorSimpleModel,
		public readonly coordinatesConverter: ICoordinatesConverter,
		public readonly cursorConfig: CursorConfiguration,
	) {}
}

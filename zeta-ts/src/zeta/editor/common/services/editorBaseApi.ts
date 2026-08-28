import { URI } from '../../../base/common/uri.js';
import { TextPosition } from '../core/position.js';
import { TextRange } from '../core/range.js';
import { SelectionDirection, TextSelection } from '../core/selection.js';

/** Value objects shared by the standalone editor and language APIs. */
export interface IEditorBaseApi {
	readonly URI: typeof URI;
	readonly TextPosition: typeof TextPosition;
	readonly TextRange: typeof TextRange;
	readonly TextSelection: typeof TextSelection;
	readonly SelectionDirection: typeof SelectionDirection;
}

export function createEditorBaseApi(): IEditorBaseApi {
	return Object.freeze({ URI, TextPosition, TextRange, TextSelection, SelectionDirection });
}

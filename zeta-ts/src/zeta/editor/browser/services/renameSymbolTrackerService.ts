import { type TextPosition, type TextRange } from '../../common/core/text.js';
import { type TextModel } from '../../common/model/textModel.js';
import { createServiceIdentifier } from '../../../platform/instantiation/common/instantiation.js';

export interface ITrackedWord {
	readonly model: TextModel;
	readonly range: TextRange;
	readonly position: TextPosition;
	readonly text: string;
}

export interface IRenameSymbolTrackerService {
	getTrackedWord(): ITrackedWord | undefined;
	setTrackedWord(word: ITrackedWord | undefined): void;
}

export const IRenameSymbolTrackerService = createServiceIdentifier<IRenameSymbolTrackerService>('renameSymbolTrackerService');

export class NullRenameSymbolTrackerService implements IRenameSymbolTrackerService {
	public getTrackedWord(): undefined {
		return undefined;
	}

	public setTrackedWord(_word: ITrackedWord | undefined): void {}
}

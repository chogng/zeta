import { type Event } from '../../../../base/common/event.js';
import { type URI } from '../../../../base/common/uri.js';
import { type Range } from '../../../common/core/range.js';
import { type LanguageFeatureRequest } from '../../../common/languages/languageFeatureRequest.js';

export interface LanguageCodeLensCommand {
	readonly id: string;
	readonly title: string;
	readonly arguments?: readonly unknown[];
}

export interface LanguageCodeLens {
	readonly range: Range;
	readonly command?: LanguageCodeLensCommand;
	readonly data?: unknown;
}

export interface LanguageCodeLensRequest extends LanguageFeatureRequest {
	readonly resource?: URI;
}

export interface LanguageCodeLensProvider {
	readonly onDidChange?: Event<void>;
	provideCodeLenses(request: LanguageCodeLensRequest, signal: AbortSignal): readonly LanguageCodeLens[] | Promise<readonly LanguageCodeLens[]>;
	resolveCodeLens?(lens: LanguageCodeLens, request: LanguageCodeLensRequest, signal: AbortSignal): LanguageCodeLens | Promise<LanguageCodeLens>;
}

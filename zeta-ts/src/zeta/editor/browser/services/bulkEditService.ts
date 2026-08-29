import { type URI } from '../../../base/common/uri.js';
import { type LanguageWorkspaceEdit, type LanguageWorkspaceTextEdit } from '../../common/languages/languageWorkspaceEdit.js';

export abstract class ResourceEdit {
	protected constructor(public readonly resource: URI) {}
}

export class ResourceTextEdit extends ResourceEdit {
	constructor(resource: URI, public readonly edit: LanguageWorkspaceTextEdit) {
		super(resource);
	}
}

export class ResourceFileEdit extends ResourceEdit {
	constructor(resource: URI, public readonly target: URI | undefined, public readonly kind: 'create' | 'rename' | 'delete') {
		super(resource);
	}
}

export interface IBulkEditOptions {
	readonly signal?: AbortSignal;
	readonly preview?: boolean;
}

export interface IBulkEditService {
	apply(edit: LanguageWorkspaceEdit, options?: IBulkEditOptions): Promise<{ readonly applied: boolean }>;
}

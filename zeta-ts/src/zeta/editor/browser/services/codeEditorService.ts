import { type Event } from '../../../base/common/event.js';
import { type IDisposable } from '../../../base/common/lifecycle.js';
import { type URI } from '../../../base/common/uri.js';
import { createServiceIdentifier } from '../../../platform/instantiation/common/instantiation.js';
import { type CodeEditorWidget } from '../widget/codeEditor/codeEditorWidget.js';

export interface ICodeEditorOpenHandler {
	(resource: URI): CodeEditorWidget | Promise<CodeEditorWidget | undefined> | undefined;
}

export interface ICodeEditorService {
	readonly onCodeEditorAdd: Event<CodeEditorWidget>;
	readonly onCodeEditorRemove: Event<CodeEditorWidget>;
	listCodeEditors(): readonly CodeEditorWidget[];
	getActiveCodeEditor(): CodeEditorWidget | undefined;
	addCodeEditor(editor: CodeEditorWidget): IDisposable;
	setActiveCodeEditor(editor: CodeEditorWidget | undefined): void;
	registerCodeEditorOpenHandler(handler: ICodeEditorOpenHandler): IDisposable;
	openCodeEditor(resource: URI): Promise<CodeEditorWidget | undefined>;
}

export const ICodeEditorService = createServiceIdentifier<ICodeEditorService>('codeEditorService');

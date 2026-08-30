import { type Event } from '../../../base/common/event.js';
import { type IDisposable } from '../../../base/common/lifecycle.js';
import { type URI } from '../../../base/common/uri.js';
import { createServiceIdentifier } from '../../../platform/instantiation/common/instantiation.js';
export interface IWidgetCodeEditor {
	getId(): string;
}

export interface IWidgetCodeEditorOpenHandler {
	(resource: URI): IWidgetCodeEditor | Promise<IWidgetCodeEditor | undefined> | undefined;
}

export interface IWidgetCodeEditorRegistry {
	readonly onCodeEditorAdd: Event<IWidgetCodeEditor>;
	readonly onCodeEditorRemove: Event<IWidgetCodeEditor>;
	listCodeEditors(): readonly IWidgetCodeEditor[];
	getActiveCodeEditor(): IWidgetCodeEditor | undefined;
	addCodeEditor(editor: IWidgetCodeEditor): IDisposable;
	setActiveCodeEditor(editor: IWidgetCodeEditor | undefined): void;
	registerCodeEditorOpenHandler(handler: IWidgetCodeEditorOpenHandler): IDisposable;
	openCodeEditor(resource: URI): Promise<IWidgetCodeEditor | undefined>;
}

export const IWidgetCodeEditorRegistry = createServiceIdentifier<IWidgetCodeEditorRegistry>('widgetCodeEditorRegistry');

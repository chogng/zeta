import { type Event } from '../../../base/common/event.js';
import { type IDisposable } from '../../../base/common/lifecycle.js';
import { type IDimension } from '../core/2d/dimension.js';
import { type ConfigurationChangedEvent, type IComputedEditorOptions, type IEditorOptions } from './editorOptions.js';

/** Mutable browser-editor configuration consumed by common view and cursor state. */
export interface IEditorConfiguration extends IDisposable {
	readonly isSimpleWidget: boolean;
	readonly contextMenuId: unknown;
	readonly options: IComputedEditorOptions;
	readonly onDidChangeFast: Event<ConfigurationChangedEvent>;
	readonly onDidChange: Event<ConfigurationChangedEvent>;
	getRawOptions(): IEditorOptions;
	updateOptions(newOptions: Readonly<IEditorOptions>): void;
	observeContainer(dimension?: IDimension): void;
	setIsDominatedByLongLines(isDominatedByLongLines: boolean): void;
	setModelLineCount(modelLineCount: number): void;
	setViewLineCount(viewLineCount: number): void;
	setReservedHeight(reservedHeight: number): void;
	setGlyphMarginDecorationLaneCount(decorationLaneCount: number): void;
}

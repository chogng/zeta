import type { Event } from '../../../base/common/event.js';
import type { IDisposable } from '../../../base/common/lifecycle.js';
import type { IDimension } from '../core/2d/dimension.js';
import type { MenuId } from '../../../platform/actions/common/actions.js';
import type { ConfigurationChangedEvent, IComputedEditorOptions, IEditorOptions } from './editorOptions.js';

/** Common editor configuration contract, matching VS Code's common layer. */
export interface IEditorConfiguration extends IDisposable {
	/** Is this a simple widget rather than a full editor? */
	readonly isSimpleWidget: boolean;
	/** Context menu identifier owned by the editor host. */
	readonly contextMenuId: MenuId;
	/** Computed editor options. */
	readonly options: IComputedEditorOptions;
	/** Fast notification for option changes. */
	readonly onDidChangeFast: Event<ConfigurationChangedEvent>;
	/** Normal notification for option changes. */
	readonly onDidChange: Event<ConfigurationChangedEvent>;
	/** Raw options merged with all calls to updateOptions. */
	getRawOptions(): IEditorOptions;
	/** Updates only the supplied option keys. */
	updateOptions(newOptions: Readonly<IEditorOptions>): void;
	/** Recomputes options with the current container dimensions. */
	observeContainer(dimension?: IDimension): void;
	/** Sets whether the current model is dominated by long lines. */
	setIsDominatedByLongLines(isDominatedByLongLines: boolean): void;
	/** Sets the current model line count. */
	setModelLineCount(modelLineCount: number): void;
	/** Sets the current view-model line count. */
	setViewLineCount(viewLineCount: number): void;
	/** Sets the height reserved above the editor. */
	setReservedHeight(reservedHeight: number): void;
	/** Sets the number of glyph-margin decoration lanes. */
	setGlyphMarginDecorationLaneCount(decorationLaneCount: number): void;
}

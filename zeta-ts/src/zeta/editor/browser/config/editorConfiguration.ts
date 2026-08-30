import { getClientArea } from '../../../base/browser/dom.js';
import { Emitter } from '../../../base/common/event.js';
import { Disposable } from '../../../base/common/lifecycle.js';
import { type IEditorConfiguration } from '../../common/config/editorConfiguration.js';
import {
	ConfigurationChangedEvent,
	EditorOption,
	editorOptionsRegistry,
	type FindComputedEditorOptionValueById,
	type IComputedEditorOptions,
	type IEditorOptions,
} from '../../common/config/editorOptions.js';
import { type FontInfo } from '../../common/config/fontInfo.js';
import { type IDimension } from '../../common/core/2d/dimension.js';

/** Browser owner for mutable editor options consumed by view and cursor configuration. */
export class EditorConfiguration extends Disposable implements IEditorConfiguration {
	private readonly _onDidChange = this._register(new Emitter<ConfigurationChangedEvent>());
	private readonly _onDidChangeFast = this._register(new Emitter<ConfigurationChangedEvent>());
	private readonly _rawOptions: IEditorOptions;
	private _containerHeight: number;
	private _reservedHeight = 0;
	private _isDominatedByLongLines = false;
	private _modelLineCount = 1;
	private _viewLineCount = 1;
	private _glyphMarginDecorationLaneCount = 1;

	readonly isSimpleWidget = false;
	readonly contextMenuId = undefined;
	readonly onDidChange = this._onDidChange.event;
	readonly onDidChangeFast = this._onDidChangeFast.event;
	readonly options: IComputedEditorOptions;

	constructor(options: IEditorOptions, private readonly _fontInfo: FontInfo, container: HTMLElement) {
		super();
		this._rawOptions = { ...options };
		this._containerHeight = Math.max(0, getClientArea(container).height);
		this.options = { get: id => this._getOption(id) };
	}

	getRawOptions(): IEditorOptions {
		return { ...this._rawOptions };
	}

	updateOptions(newOptions: Readonly<IEditorOptions>): void {
		const changed = Array.from({ length: editorOptionsRegistry.length }, () => false);
		for (const [name, value] of Object.entries(newOptions)) {
			if (Object.is((this._rawOptions as Record<string, unknown>)[name], value)) continue;
			(this._rawOptions as Record<string, unknown>)[name] = value;
			const option = editorOptionsRegistry.find(candidate => candidate.name === name);
			if (option) changed[option.id] = true;
		}
		this._fireChange(changed);
	}

	observeContainer(dimension?: IDimension): void {
		const height = Math.max(0, dimension?.height ?? this._containerHeight);
		if (height === this._containerHeight) return;
		this._containerHeight = height;
		this._fireLayoutChange();
	}

	setIsDominatedByLongLines(isDominatedByLongLines: boolean): void {
		if (this._isDominatedByLongLines === isDominatedByLongLines) return;
		this._isDominatedByLongLines = isDominatedByLongLines;
		this._fireLayoutChange();
	}

	setModelLineCount(modelLineCount: number): void {
		if (this._modelLineCount === modelLineCount) return;
		this._modelLineCount = modelLineCount;
		this._fireLayoutChange();
	}

	setViewLineCount(viewLineCount: number): void {
		if (this._viewLineCount === viewLineCount) return;
		this._viewLineCount = viewLineCount;
		this._fireLayoutChange();
	}

	setReservedHeight(reservedHeight: number): void {
		if (this._reservedHeight === reservedHeight) return;
		this._reservedHeight = reservedHeight;
		this._fireLayoutChange();
	}

	setGlyphMarginDecorationLaneCount(decorationLaneCount: number): void {
		if (this._glyphMarginDecorationLaneCount === decorationLaneCount) return;
		this._glyphMarginDecorationLaneCount = decorationLaneCount;
		this._fireLayoutChange();
	}

	private _getOption<T extends EditorOption>(id: T): FindComputedEditorOptionValueById<T> {
		if (id === EditorOption.fontInfo) return this._fontInfo as unknown as FindComputedEditorOptionValueById<T>;
		if (id === EditorOption.layoutInfo) return { height: Math.max(0, this._containerHeight - this._reservedHeight) } as unknown as FindComputedEditorOptionValueById<T>;
		const option = editorOptionsRegistry[id];
		if (!option) throw new ReferenceError(`Missing editor option ${id}`);
		return option.validate((this._rawOptions as Record<string, unknown>)[option.name]) as FindComputedEditorOptionValueById<T>;
	}

	private _fireChange(changed: boolean[]): void {
		if (!changed.some(Boolean)) return;
		const event = new ConfigurationChangedEvent(changed);
		this._onDidChangeFast.fire(event);
		this._onDidChange.fire(event);
	}

	private _fireLayoutChange(): void {
		const changed = Array.from({ length: editorOptionsRegistry.length }, () => false);
		changed[EditorOption.layoutInfo] = true;
		this._fireChange(changed);
	}
}

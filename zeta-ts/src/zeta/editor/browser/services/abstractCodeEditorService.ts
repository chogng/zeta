import { Emitter } from '../../../base/common/event.js';
import { Disposable, toDisposable } from '../../../base/common/lifecycle.js';
import { LinkedList } from '../../../base/common/linkedList.js';
import { type URI } from '../../../base/common/uri.js';
import { type ITextResourceEditorInput } from '../../../platform/editor/common/editor.js';
import { type ICodeEditor, type IDiffEditor } from '../editorBrowser.js';
import { isThemeColor, type IDecorationRenderOptions, type IThemeDecorationRenderOptions } from '../../common/editorCommon.js';
import { type IModelDecorationOptions, type ITextModel } from '../../common/model.js';
import { type ICodeEditorOpenHandler, type ICodeEditorService } from './codeEditorService.js';

interface Decoration {
	readonly description: string;
	readonly className: string;
	readonly options: IDecorationRenderOptions;
	readonly parentKey?: string;
	readonly style?: HTMLStyleElement;
	refs: number;
}

/** Owns editor discovery and the state shared by all code editor instances. */
export abstract class AbstractCodeEditorService extends Disposable implements ICodeEditorService {
	declare readonly _serviceBrand: undefined;

	private readonly willCreateCodeEditorEmitter = this._register(new Emitter<void>());
	private readonly codeEditorAddEmitter = this._register(new Emitter<ICodeEditor>());
	private readonly codeEditorRemoveEmitter = this._register(new Emitter<ICodeEditor>());
	private readonly willCreateDiffEditorEmitter = this._register(new Emitter<void>());
	private readonly diffEditorAddEmitter = this._register(new Emitter<IDiffEditor>());
	private readonly diffEditorRemoveEmitter = this._register(new Emitter<IDiffEditor>());
	private readonly transientChangeEmitter = this._register(new Emitter<ITextModel>());
	private readonly decorationRegisteredEmitter = this._register(new Emitter<string>());
	private readonly codeEditors = new Map<string, ICodeEditor>();
	private readonly diffEditors = new Map<string, IDiffEditor>();
	private readonly openHandlers = new LinkedList<ICodeEditorOpenHandler>();
	private readonly decorations = new Map<string, Decoration>();
	private readonly modelProperties = new Map<string, Map<string, unknown>>();
	private readonly transientProperties = new WeakMap<ITextModel, Map<string, unknown>>();
	private readonly transientModels = new Map<string, ITextModel>();
	private activeEditor: ICodeEditor | null = null;

	readonly onWillCreateCodeEditor = this.willCreateCodeEditorEmitter.event;
	readonly onCodeEditorAdd = this.codeEditorAddEmitter.event;
	readonly onCodeEditorRemove = this.codeEditorRemoveEmitter.event;
	readonly onWillCreateDiffEditor = this.willCreateDiffEditorEmitter.event;
	readonly onDiffEditorAdd = this.diffEditorAddEmitter.event;
	readonly onDiffEditorRemove = this.diffEditorRemoveEmitter.event;
	readonly onDidChangeTransientModelProperty = this.transientChangeEmitter.event;
	readonly onDecorationTypeRegistered = this.decorationRegisteredEmitter.event;

	willCreateCodeEditor(): void {
		this.willCreateCodeEditorEmitter.fire();
	}

	addCodeEditor(editor: ICodeEditor): void {
		const id = editor.getId();
		if (this.codeEditors.has(id)) {
			throw new Error(`Code editor already exists: ${id}`);
		}
		this.codeEditors.set(id, editor);
		this.activeEditor = editor;
		this.codeEditorAddEmitter.fire(editor);
	}

	removeCodeEditor(editor: ICodeEditor): void {
		if (!this.codeEditors.delete(editor.getId())) {
			return;
		}
		if (this.activeEditor === editor) {
			this.activeEditor = this.lastCodeEditor();
		}
		this.codeEditorRemoveEmitter.fire(editor);
	}

	listCodeEditors(): readonly ICodeEditor[] {
		return [...this.codeEditors.values()];
	}

	willCreateDiffEditor(): void {
		this.willCreateDiffEditorEmitter.fire();
	}

	addDiffEditor(editor: IDiffEditor): void {
		const id = editor.getId();
		if (this.diffEditors.has(id)) {
			throw new Error(`Diff editor already exists: ${id}`);
		}
		this.diffEditors.set(id, editor);
		this.diffEditorAddEmitter.fire(editor);
	}

	removeDiffEditor(editor: IDiffEditor): void {
		if (this.diffEditors.delete(editor.getId())) {
			this.diffEditorRemoveEmitter.fire(editor);
		}
	}

	listDiffEditors(): readonly IDiffEditor[] {
		return [...this.diffEditors.values()];
	}

	getFocusedCodeEditor(): ICodeEditor | null {
		let widget: ICodeEditor | null = null;
		for (const editor of this.codeEditors.values()) {
			if (editor.hasTextFocus()) {
				return editor;
			}
			if (editor.hasWidgetFocus()) {
				widget = editor;
			}
		}
		return widget;
	}

	getActiveCodeEditor(): ICodeEditor | null {
		return this.getFocusedCodeEditor() ?? this.activeEditor;
	}

	registerDecorationType(description: string, key: string, options: IDecorationRenderOptions, parentKey?: string, editor?: ICodeEditor) {
		const current = this.decorations.get(key);
		if (current) {
			current.refs++;
			return toDisposable(() => this.removeDecorationType(key));
		}

		const className = `zeta-decoration-${safeName(key)}`;
		const style = createStyle(editor);
		const decoration: Decoration = { description, className, options, parentKey, style, refs: 1 };
		style?.appendChild(document.createTextNode(renderRule(className, this.mergeDecorationOptions(decoration))));
		this.decorations.set(key, decoration);
		this.decorationRegisteredEmitter.fire(key);
		return toDisposable(() => this.removeDecorationType(key));
	}

	listDecorationTypes(): string[] {
		return [...this.decorations.keys()];
	}

	removeDecorationType(key: string): void {
		const decoration = this.decorations.get(key);
		if (!decoration || --decoration.refs > 0) {
			return;
		}
		this.decorations.delete(key);
		decoration.style?.remove();
		for (const editor of this.codeEditors.values()) {
			editor.removeDecorationsByType(key);
		}
	}

	resolveDecorationOptions(key: string, writable: boolean): IModelDecorationOptions {
		const decoration = this.decorations.get(key);
		if (!decoration) {
			throw new Error(`Unknown decoration type: ${key}`);
		}
		const options = this.mergeDecorationOptions(decoration);
		const result: IModelDecorationOptions = {
			description: decoration.description,
			className: decoration.className,
			isWholeLine: options.isWholeLine,
			stickiness: options.rangeBehavior,
			fontFamily: options.fontFamily,
			fontSize: options.fontSize,
			fontWeight: options.fontWeight,
			fontStyle: options.fontStyle,
			overviewRuler: options.overviewRulerColor ? {
				color: options.overviewRulerColor,
				position: options.overviewRulerLane ?? 7,
			} : undefined,
		};
		return writable ? { ...result } : result;
	}

	resolveDecorationCSSRules(key: string): CSSRuleList | null {
		return this.decorations.get(key)?.style?.sheet?.cssRules ?? null;
	}

	setModelProperty(resource: URI, key: string, value: unknown): void {
		setValue(this.modelProperties, resource.toString(), key, value);
	}

	getModelProperty(resource: URI, key: string): unknown {
		return this.modelProperties.get(resource.toString())?.get(key);
	}

	setTransientModelProperty(model: ITextModel, key: string, value: unknown): void {
		let values = this.transientProperties.get(model);
		if (!values) {
			values = new Map();
			this.transientProperties.set(model, values);
			const uri = model.uri.toString();
			this.transientModels.set(uri, model);
			this._register(model.onWillDispose(() => this.transientModels.delete(uri)));
		}
		if (values.get(key) === value) {
			return;
		}
		value === undefined ? values.delete(key) : values.set(key, value);
		this.transientChangeEmitter.fire(model);
	}

	getTransientModelProperty(model: ITextModel, key: string): unknown {
		return this.transientProperties.get(model)?.get(key);
	}

	getTransientModelProperties(model: ITextModel): [string, unknown][] | undefined {
		const values = this.transientProperties.get(model);
		return values ? [...values] : undefined;
	}

	async openCodeEditor(input: ITextResourceEditorInput, source: ICodeEditor | null, sideBySide?: boolean): Promise<ICodeEditor | null> {
		for (const handler of this.openHandlers) {
			const editor = await handler(input, source, sideBySide);
			if (editor) {
				this.activeEditor = editor;
				return editor;
			}
		}
		return null;
	}

	registerCodeEditorOpenHandler(handler: ICodeEditorOpenHandler) {
		return toDisposable(this.openHandlers.unshift(handler));
	}

	private lastCodeEditor(): ICodeEditor | null {
		let result: ICodeEditor | null = null;
		for (const editor of this.codeEditors.values()) {
			result = editor;
		}
		return result;
	}

	private mergeDecorationOptions(decoration: Decoration): IDecorationRenderOptions {
		if (!decoration.parentKey) {
			return decoration.options;
		}
		const parent = this.decorations.get(decoration.parentKey);
		return parent ? { ...this.mergeDecorationOptions(parent), ...decoration.options } : decoration.options;
	}
}

function setValue(store: Map<string, Map<string, unknown>>, id: string, key: string, value: unknown): void {
	let values = store.get(id);
	if (!values) {
		values = new Map();
		store.set(id, values);
	}
	value === undefined ? values.delete(key) : values.set(key, value);
	if (values.size === 0) {
		store.delete(id);
	}
}

function safeName(value: string): string {
	return value.replace(/[^a-zA-Z0-9_-]/g, '-');
}

function createStyle(editor?: ICodeEditor): HTMLStyleElement | undefined {
	if (typeof document === 'undefined') {
		return undefined;
	}
	const style = document.createElement('style');
	const root = editor?.getContainerDomNode().getRootNode();
	if (root instanceof ShadowRoot) {
		root.appendChild(style);
	} else {
		document.head.appendChild(style);
	}
	return style;
}

function renderRule(className: string, options: IThemeDecorationRenderOptions): string {
	const css: string[] = [];
	addCss(css, 'background-color', options.backgroundColor);
	addCss(css, 'color', options.color);
	addCss(css, 'border', options.border);
	addCss(css, 'border-color', options.borderColor);
	addCss(css, 'border-radius', options.borderRadius);
	addCss(css, 'border-style', options.borderStyle);
	addCss(css, 'border-width', options.borderWidth);
	addCss(css, 'outline', options.outline);
	addCss(css, 'outline-color', options.outlineColor);
	addCss(css, 'outline-style', options.outlineStyle);
	addCss(css, 'outline-width', options.outlineWidth);
	addCss(css, 'font-family', options.fontFamily);
	addCss(css, 'font-size', options.fontSize);
	addCss(css, 'font-style', options.fontStyle);
	addCss(css, 'font-weight', options.fontWeight);
	addCss(css, 'line-height', options.lineHeight);
	addCss(css, 'text-decoration', options.textDecoration);
	addCss(css, 'cursor', options.cursor);
	addCss(css, 'opacity', options.opacity);
	addCss(css, 'letter-spacing', options.letterSpacing);
	return `.${className}{${css.join(';')}}`;
}

function addCss(target: string[], name: string, value: string | number | { id: string } | undefined): void {
	if (value === undefined) {
		return;
	}
	const text = isThemeColor(value) ? `var(--vscode-${value.id.replace(/\./g, '-')})` : String(value);
	target.push(`${name}:${text}`);
}

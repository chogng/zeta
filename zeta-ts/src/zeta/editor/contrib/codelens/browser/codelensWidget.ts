import './media/codelens.css';
import { addDisposableListener, h } from '../../../../base/browser/dom.js';
import { Disposable } from '../../../../base/common/lifecycle.js';
import { localize } from '../../../../nls.js';
import { type EditorViewport, type EditorViewZone, type EditorViewZoneHandle } from '../../../browser/view.js';
import { type LanguageCodeLens, type LanguageCodeLensCommand } from '../common/codelens.js';
import { type CodeLensItem } from './codelens.js';

export type ExecuteCodeLensCommand = (command: LanguageCodeLensCommand) => void;

/** Owns the stable DOM and interactions for one line of code lenses. */
export class CodeLensWidget extends Disposable {
	public readonly domNode: HTMLDivElement;
	private items: readonly CodeLensItem[];
	private currentCommands: readonly LanguageCodeLensCommand[] = [];
	private commandsResolved = false;
	private readonly viewZone: EditorViewZone;
	private readonly viewZoneHandle: EditorViewZoneHandle;

	public constructor(private readonly viewport: EditorViewport, items: readonly CodeLensItem[], private readonly executeCommand?: ExecuteCodeLensCommand) {
		super();
		this.items = items;
		this.domNode = h(viewport.element.ownerDocument, 'div');
		this.domNode.className = 'stanza-editor-widget stanza-editor-codelens';
		this.domNode.setAttribute('role', 'group');
		this.domNode.setAttribute('aria-label', localize('zeta.editor.codeLens', 'commands', 'CodeLens commands'));
		this.viewZone = {
			afterLineIndex: this.afterVisualLineIndex,
			heightInPixels: this.codeLensHeight,
			domNode: this.domNode,
		};
		this.viewZoneHandle = this._register(viewport.addViewZone(this.viewZone));
		this._register(addDisposableListener<MouseEvent>(this.domNode, 'click', event => {
			const button = (event.target as Element | null)?.closest<HTMLButtonElement>('.stanza-editor-codelens-command');
			if (!(button instanceof this.domNode.ownerDocument.defaultView!.HTMLButtonElement) || button.parentElement !== this.domNode || !this.executeCommand) return;
			const command = this.currentCommands[Number(button.dataset.commandIndex)];
			if (command) this.executeCommand(command);
		}));
		this.render(this.initialSymbols);
		this.layout();
	}

	public get codeLensItems(): readonly CodeLensItem[] {
		return this.items;
	}

	public get needsResolve(): boolean {
		return !this.commandsResolved && this.items.some(item => !item.symbol.command && item.provider.resolveCodeLens !== undefined);
	}

	public updateCodeLensItems(items: readonly CodeLensItem[]): void {
		this.items = items;
		this.commandsResolved = false;
		this.render(this.initialSymbols);
		this.layout();
	}

	public updateResolvedCodeLensItems(items: readonly CodeLensItem[]): void {
		this.items = items;
		this.commandsResolved = true;
		this.render(this.initialSymbols);
	}

	public layout(): void {
		this.viewZone.afterLineIndex = this.afterVisualLineIndex;
		this.viewZone.heightInPixels = this.codeLensHeight;
		this.viewZoneHandle.layout();
		const coordinates = this.viewport.getPositionContentCoordinates(this.items[0]!.symbol.range.start);
		this.domNode.style.left = `${Math.max(4, coordinates.left)}px`;
	}

	public isVisible(): boolean {
		const layout = this.viewport.viewportLayout;
		return this.viewZoneHandle.top + this.viewZoneHandle.heightInPixels >= layout.scrollPosition.top && this.viewZoneHandle.top <= layout.scrollPosition.top + layout.viewportSize.height;
	}

	private get afterVisualLineIndex(): number {
		return this.viewport.getVisualLineProjection().visualLineIndexAt(this.items[0]!.symbol.range.start) - 1;
	}

	private get codeLensHeight(): number {
		return Math.max(11, Math.floor(this.viewport.viewportLayout.lineHeight * 0.7));
	}

	private get initialSymbols(): readonly (LanguageCodeLens | undefined)[] {
		return this.items.map(item => item.symbol.command ? item.symbol : undefined);
	}

	private render(symbols: readonly (LanguageCodeLens | undefined)[]): void {
		const commands = symbols.flatMap(symbol => symbol?.command ? [symbol.command] : []);
		this.currentCommands = commands;
		if (commands.length === 0) {
			this.domNode.replaceChildren();
			this.domNode.hidden = true;
			return;
		}
		const children: HTMLElement[] = [];
		for (let index = 0; index < commands.length; index += 1) {
			if (index > 0) {
				const separator = h(this.domNode.ownerDocument, 'span');
				separator.className = 'stanza-editor-codelens-separator';
				separator.textContent = '\u00a0|\u00a0';
				separator.setAttribute('aria-hidden', 'true');
				children.push(separator);
			}
			const command = commands[index]!;
			const element = h(this.domNode.ownerDocument, this.executeCommand && command.id.length > 0 ? 'button' : 'span');
			element.className = 'stanza-editor-codelens-command';
			element.textContent = command.title.trim();
			if (element instanceof this.domNode.ownerDocument.defaultView!.HTMLButtonElement) {
				element.type = 'button';
				element.dataset.commandIndex = String(index);
			}
			children.push(element);
		}
		this.domNode.replaceChildren(...children);
		this.domNode.hidden = false;
	}
}

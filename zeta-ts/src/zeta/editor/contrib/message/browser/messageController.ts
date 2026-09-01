import { addDisposableListener } from '../../../../base/browser/dom.js';
import { MarkdownElement } from '../../../../base/browser/markdownRenderer.js';
import { disposableWindowTimeout } from '../../../../base/browser/scheduler.js';
import type { IMarkdownString } from '../../../../base/common/htmlContent.js';
import { isMarkdownString } from '../../../../base/common/htmlContent.js';
import { Disposable, DisposableStore, MutableDisposable, type IDisposable, toDisposable } from '../../../../base/common/lifecycle.js';
import { IContextKeyService, RawContextKey, type IContextKey } from '../../../../platform/contextkey/common/contextkey.js';
import { ServiceConstructionDescriptor } from '../../../../platform/instantiation/common/instantiation.js';
import { ContentWidgetPositionPreference, type ICodeEditor, type IContentWidget, type IContentWidgetPosition } from '../../../browser/editorBrowser.js';
import { EditorCommand, EditorContributionInstantiation, registerEditorCommand, registerTextEditorCapabilityContribution, type TextEditorContributionContext } from '../../../browser/editorExtensions.js';
import { type IPosition, Position } from '../../../common/core/position.js';
import type { IEditorContribution } from '../../../common/editorCommon.js';
import { PositionAffinity } from '../../../common/model.js';
import './messageController.css';

/** Owns the active editor-positioned message and its dismissal lifecycle. */
export class MessageController extends Disposable implements IEditorContribution {
	public static readonly ID = 'editor.contrib.messageController';
	public static readonly MESSAGE_VISIBLE = new RawContextKey<boolean>('messageVisible', false);

	public static get(editor: ICodeEditor): MessageController | null {
		return editor.getContribution<MessageController>(MessageController.ID);
	}

	private readonly visibleKey: IContextKey<boolean> | undefined;
	private readonly widget = this._register(new MutableDisposable<MessageWidget>());
	private readonly closeTimer = this._register(new MutableDisposable<IDisposable>());
	private readonly listeners = this._register(new DisposableStore());
	private visible = false;
	private mouseOver = false;

	constructor(private readonly context: TextEditorContributionContext) {
		super();
		this.visibleKey = context.instantiationService.getOptional(IContextKeyService)?.createKey(MessageController.MESSAGE_VISIBLE.key, false);
		if (this.visibleKey) this._register(toDisposable(() => this.visibleKey?.reset()));
		this._register(addDisposableListener<KeyboardEvent>(context.view.element, 'keydown', event => {
			if (!this.visible || event.defaultPrevented || event.key !== 'Escape') return;
			event.preventDefault();
			event.stopPropagation();
			this.closeMessage();
		}));
	}

	public isVisible(): boolean {
		return this.visible;
	}

	public showMessage(message: IMarkdownString | string, position: IPosition): void {
		const text = isMarkdownString(message) ? message.value : message;
		if (typeof text !== 'string' || text.trim().length === 0) throw new TypeError('Editor message must not be empty');
		this.context.viewport.announceAccessibilityStatus(text);
		this.closeTimer.clear();
		this.listeners.clear();
		this.widget.clear();
		this.mouseOver = false;

		let content: string | HTMLElement = text;
		let contentOwner: IDisposable | undefined;
		if (isMarkdownString(message)) {
			const markdown = new MarkdownElement({
				ownerDocument: this.context.viewport.element.ownerDocument,
				markdown: message.value,
				linkHandler: target => {
					this.closeMessage();
					void this.context.options.onOpenLink?.(target);
				},
			});
			content = markdown.element;
			contentOwner = markdown;
		}
		const widget = new MessageWidget(this.context.editor, this.context.viewport, position, content, contentOwner);
		this.widget.value = widget;
		this.setVisible(true);
		this.listenForDismissal(position, widget);
	}

	public closeMessage(): void {
		this.setVisible(false);
		this.mouseOver = false;
		this.listeners.clear();
		this.closeTimer.clear();
		const widget = this.widget.value;
		if (!widget) return;
		widget.getDomNode().classList.add('fadeOut');
		const targetWindow = widget.getDomNode().ownerDocument.defaultView;
		if (!targetWindow) {
			this.widget.clear();
			return;
		}
		this.closeTimer.value = disposableWindowTimeout(targetWindow, () => this.widget.clear(), 100);
	}

	private listenForDismissal(position: IPosition, widget: MessageWidget): void {
		this.listeners.add(this.context.selectionController.onDidChange(() => this.closeMessage()));
		this.listeners.add(this.context.model.onDidChangeContent(() => this.closeMessage()));
		this.listeners.add(addDisposableListener(widget.getDomNode(), 'mouseenter', () => { this.mouseOver = true; }));
		this.listeners.add(addDisposableListener(widget.getDomNode(), 'mouseleave', () => { this.mouseOver = false; }));
		this.listeners.add(addDisposableListener<FocusEvent>(this.context.view.element, 'focusout', () => {
			const targetWindow = this.context.view.element.ownerDocument.defaultView;
			if (!targetWindow) return;
			this.listeners.add(disposableWindowTimeout(targetWindow, () => {
				const active = this.context.view.element.ownerDocument.activeElement;
				if (!this.mouseOver && active !== null && !widget.getDomNode().contains(active)) this.closeMessage();
			}, 0));
		}));
		this.listeners.add(addDisposableListener<PointerEvent>(this.context.viewport.element, 'pointermove', event => {
			const target = this.context.viewport.getNearestTargetAtClientPoint(event);
			if (target && Math.abs(target.position.lineNumber - position.lineNumber) > 3) this.closeMessage();
		}));
	}

	private setVisible(visible: boolean): void {
		this.visible = visible;
		if (visible) this.visibleKey?.set(true);
		else this.visibleKey?.reset();
	}
}

class MessageWidget extends Disposable implements IContentWidget {
	public readonly allowEditorOverflow = true;
	public readonly suppressMouseDown = false;
	private readonly domNode: HTMLDivElement;

	constructor(
		private readonly editor: ICodeEditor,
		viewport: TextEditorContributionContext['viewport'],
		private readonly position: IPosition,
		content: HTMLElement | string,
		contentOwner?: IDisposable,
	) {
		super();
		if (contentOwner) this._register(contentOwner);
		viewport.revealPosition(Position.lift(position));
		const document = editor.getContainerDomNode().ownerDocument;
		this.domNode = document.createElement('div');
		this.domNode.className = 'stanza-editor-overlay-message fadeIn';
		const message = document.createElement('div');
		message.className = 'message';
		if (typeof content === 'string') message.textContent = content;
		else message.append(content);
		this.domNode.append(message);
		editor.addContentWidget(this);
		this._register(toDisposable(() => editor.removeContentWidget(this)));
		editor.layoutContentWidget(this);
	}

	public getId(): string {
		return 'messageoverlay';
	}

	public getDomNode(): HTMLElement {
		return this.domNode;
	}

	public getPosition(): IContentWidgetPosition {
		return {
			position: this.position,
			preference: [ContentWidgetPositionPreference.ABOVE, ContentWidgetPositionPreference.BELOW],
			positionAffinity: PositionAffinity.Right,
		};
	}

	public afterRender(position: ContentWidgetPositionPreference | null): void {
		this.domNode.classList.toggle('below', position === ContentWidgetPositionPreference.BELOW);
	}
}

const MessageCommand = EditorCommand.bindToContribution<MessageController>(MessageController.get);
registerEditorCommand(new MessageCommand({
	id: 'leaveEditorMessage',
	precondition: MessageController.MESSAGE_VISIBLE.isEqualTo(true),
	handler: controller => controller.closeMessage(),
}));

registerTextEditorCapabilityContribution({
	id: MessageController.ID,
	runtime: {
		descriptor: new ServiceConstructionDescriptor(MessageController),
		instantiation: EditorContributionInstantiation.Lazy,
	},
});

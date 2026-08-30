import { Position } from "../../../../common/core/position.js";
import "../media/inlineCompletions.css";
import { registerTextEditorCapabilityContribution } from "../../../../browser/editorExtensions.js";
import { addDisposableListener, stopEvent, h } from "../../../../../base/browser/dom.js";
import { Disposable, toDisposable } from "../../../../../base/common/lifecycle.js";
import { createEditorEditCommand } from "../../../../common/commands/editorCommand.js";
import { Range } from "../../../../common/core/range.js";
import { type CursorsController } from "../../../../common/cursor/cursor.js";
import { InlineCompletionProviderService } from "../../../../browser/services/inlineCompletionProviderService.js";
import { type LanguageInlineCompletionItem } from "../../common/inlineCompletions.js";
import { type View } from "../../../../browser/view.js";
import { isCompletionsEnablementEnabled } from "../../../../common/services/ownedCompletionsEnablement.js";
import { type Event } from '../../../../../base/common/event.js';
import { type EditorCommandEvent } from '../../../../browser/editorExtensions.js';
import { TriggerInlineEditCommandsRegistry } from '../../../../browser/triggerInlineEditCommandsRegistry.js';

/** Owns ghost-text projection and explicit acceptance of one inline completion. */
export class InlineCompletionsController extends Disposable {
	private readonly element: HTMLSpanElement;
	private request: AbortController | undefined;
	private item: LanguageInlineCompletionItem | undefined;

	constructor(private readonly input: HTMLElement, private readonly viewport: View, private readonly selections: CursorsController, private readonly service: InlineCompletionProviderService, private readonly languageId: string, onDidExecuteCommand?: Event<EditorCommandEvent>, private readonly onError: (error: unknown) => void = error => console.error("Stanza inline completion failed", error)) {
		super();
		const element = this.element = h(viewport.element.ownerDocument, "span");
		element.className = "stanza-editor-inline-completion";
		element.hidden = true;
		viewport.element.append(element);
		this._register(toDisposable(() => element.remove()));
		this._register(addDisposableListener(input, "keydown", event => {
			if (event.defaultPrevented || event.isComposing || !event.ctrlKey || !event.altKey || event.key !== " ") return;
			stopEvent(event);
			void this.refresh("explicit");
		}));
		this._register(addDisposableListener(input, "keydown", event => {
			if (event.defaultPrevented || event.isComposing || !this.item || event.key !== "Enter" || !event.altKey) return;
			stopEvent(event);
			this.accept();
		}));
		this._register(selections.onDidChange(() => this.clear()));
		this._register(viewport.onDidChangeLayout(() => this.render()));
		this._register(viewport.textModel.onDidChange(() => this.clear()));
		if (onDidExecuteCommand) {
			const triggerCommands = new Set(TriggerInlineEditCommandsRegistry.getRegisteredCommands());
			this._register(onDidExecuteCommand(event => {
				if (!triggerCommands.has(event.commandId)) return;
				void this.refresh('automatic');
			}));
		}
	}

	private async refresh(triggerKind: "automatic" | "explicit"): Promise<void> {
		const selection = this.selections.selections.primary;
		if (!selection.isEmpty()) return;
		this.request?.abort();
		const request = this.request = new AbortController();
		try {
			const items = await this.service.provideInlineCompletions(this.languageId, selection.getPosition(), triggerKind, request.signal);
			if (request.signal.aborted) return;
			this.item = items[0];
			this.render();
		} catch (error) {
			if (!request.signal.aborted) this.onError(error);
		}
	}

	private render(): void {
		const item = this.item;
		if (!item) {
			this.element.hidden = true;
			return;
		}
		const selection = this.selections.selections.primary;
		const range = item.range ?? Range.fromPositions(selection.getPosition());
		const coordinates = this.viewport.getPositionContentCoordinates(range.getStartPosition());
		const scroll = this.viewport.viewportLayout.scrollPosition;
		this.element.textContent = item.insertText;
		this.element.style.left = `${coordinates.left - scroll.left}px`;
		this.element.style.top = `${coordinates.top - scroll.top}px`;
		this.element.hidden = false;
	}

	private accept(): void {
		const item = this.item;
		if (!item) return;
		const selection = this.selections.selections.primary;
		const edits = [...(item.additionalTextEdits ?? []), { range: item.range ?? Range.fromPositions(selection.getPosition()), text: item.insertText }].sort((left, right) => Position.compare(Range.lift(left.range).getStartPosition(), Range.lift(right.range).getStartPosition()));
		const command = createEditorEditCommand(this.viewport.textModel, this.selections.selections, edits);
		if (command) this.selections.execute(command);
		this.clear();
	}

	private clear(): void {
		this.request?.abort();
		this.request = undefined;
		this.item = undefined;
		this.element.hidden = true;
		this.element.textContent = "";
	}
}

registerTextEditorCapabilityContribution({ id: "editor.contrib.inlineCompletions", install: context => {
	if (context.kind !== "text" || (context.options.inlineCompletions !== undefined && !isCompletionsEnablementEnabled(context.options.inlineCompletions, context.languageId))) return;
	const service = context.register(new InlineCompletionProviderService(context.model, context.languageFeaturesService.inlineCompletionsProvider));
	context.register(new InlineCompletionsController(context.view.element, context.viewport, context.selections, service, context.languageId, context.onDidExecuteCommand, context.onLanguageError));
} });

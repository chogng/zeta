import "./media/parameterHints.css";
import { registerEditorContribution } from "../../../browser/editorContribution.js";
import { addDisposableListener, stopEvent, h } from "../../../../base/browser/dom.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type ParameterHintsService, type LanguageParameterHints, type LanguageParameterHintsContext } from "../common/parameterHints.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";

/** Routes the signature-help shortcut and owns the accessible parameter widget. */
export class ParameterHintsController extends DisposableOwner {
	private readonly element: HTMLDivElement;
	private request: AbortController | undefined;

	constructor(private readonly input: HTMLTextAreaElement, private readonly viewport: EditorViewport, private readonly selections: EditorSelectionController, private readonly service: ParameterHintsService, private readonly languageId: string, private readonly onError: (error: unknown) => void = error => console.error("Aster parameter hints failed", error)) {
		super();
		this.element = h(viewport.element.ownerDocument, "div");
		this.element.className = "aster-editor-parameter-hints";
		this.element.hidden = true;
		this.element.setAttribute("role", "dialog");
		this.element.setAttribute("aria-label", "Parameter hints");
		viewport.element.append(this.element);
		this.defer(() => this.element.remove());
		this.own(addDisposableListener(input, "keydown", event => {
			if (event.defaultPrevented || event.isComposing || !event.shiftKey || event.altKey || (!event.ctrlKey && !event.metaKey) || event.key !== " ") return;
			stopEvent(event);
			void this.refresh();
		}));
		this.own(addDisposableListener(input, "keydown", event => {
			if (event.key !== "Escape" || this.element.hidden) return;
			stopEvent(event);
			this.hide();
		}));
		this.own(viewport.textModel.onDidChange(change => {
			const inserted = change.changes.length === 1 ? change.changes[0]!.text : "";
			const triggerIndex = Math.max(inserted.lastIndexOf("("), inserted.lastIndexOf(","));
			const trigger = triggerIndex >= 0 ? inserted[triggerIndex] : undefined;
			if (trigger === "(" || trigger === ",") {
				queueMicrotask(() => void this.refresh({ kind: "triggerCharacter", triggerCharacter: trigger }));
			} else if (!this.element.hidden) {
				queueMicrotask(() => void this.refresh({ kind: "contentChange" }));
			} else {
				this.hide();
			}
		}));
		this.own(selections.onDidChange(() => this.hide()));
		this.own(viewport.onDidChangeLayout(() => this.position()));
	}

	private async refresh(context: LanguageParameterHintsContext = { kind: "invoke" }): Promise<void> {
		this.request?.abort();
		const request = this.request = new AbortController();
		try {
			const position = this.selections.selections.primary.active;
			const hints = await this.service.provideParameterHints(this.languageId, position, context, request.signal);
			if (request.signal.aborted) return;
			if (!hints) { this.hide(); return; }
			this.render(hints);
		} catch (error) {
			if (!request.signal.aborted) this.onError(error);
		}
	}

	private render(hints: LanguageParameterHints): void {
		this.element.replaceChildren(...hints.signatures.map((signature, index) => {
			const node = h(this.element.ownerDocument, "div");
			node.className = `aster-editor-parameter-hints-signature${hints.activeSignature === index ? " active" : ""}`;
			const activeParameter = (hints.activeSignature ?? 0) === index && signature.activeParameter !== undefined ? signature.parameters[signature.activeParameter] : undefined;
			const activeStart = activeParameter ? signature.label.indexOf(activeParameter.label) : -1;
			if (activeParameter && activeStart >= 0) {
				const parameter = h(this.element.ownerDocument, "strong");
				parameter.className = "aster-editor-parameter-hints-parameter active";
				parameter.textContent = activeParameter.label;
				node.append(signature.label.slice(0, activeStart), parameter, signature.label.slice(activeStart + activeParameter.label.length));
			} else {
				node.textContent = signature.label;
			}
			if (signature.documentation) node.title = signature.documentation;
			return node;
		}));
		this.position();
		this.element.hidden = false;
	}

	private position(): void {
		if (this.element.hidden) return;
		const position = this.selections.selections.primary.active;
		const coordinates = this.viewport.getPositionContentCoordinates(position);
		const scroll = this.viewport.viewportLayout.scrollPosition;
		this.element.style.left = `${Math.max(8, coordinates.left - scroll.left)}px`;
		this.element.style.top = `${Math.max(8, coordinates.top - scroll.top + coordinates.height + 4)}px`;
	}

	private hide(): void {
		this.request?.abort();
		this.request = undefined;
		this.element.hidden = true;
		this.element.replaceChildren();
	}
}

registerEditorContribution({ id: "editor.contrib.parameterHints", install: context => {
	if (context.kind !== "text" || context.options.parameterHints === false) return;
	const service = context.own(context.languageFeaturesService.createParameterHintsService(context.model, context.options.input.resource));
	context.own(new ParameterHintsController(context.textInput.element, context.viewport, context.selections, service, context.languageId, context.onLanguageError));
} });

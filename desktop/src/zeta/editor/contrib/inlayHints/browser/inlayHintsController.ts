import "./media/inlayHints.css";
import { registerEditorContribution } from "../../../browser/editorContribution.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { TextPosition, TextRange } from "../../../common/core/text.js";
import { type InlayHintsService, type LanguageInlayHint } from "../common/inlayHints.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";

/** Projects versioned inlay hints into lightweight editor-local inline nodes. */
export class InlayHintsController extends DisposableOwner {
  private hints: readonly LanguageInlayHint[] = [];
  private request: AbortController | undefined;

  constructor(private readonly viewport: EditorViewport, private readonly service: InlayHintsService, private readonly languageId: string, private readonly onError: (error: unknown) => void = error => console.error("Aster inlay hints failed", error)) {
    super();
    this.own(viewport.onDidChangeLayout(() => this.render()));
    this.own(viewport.textModel.onDidChange(() => void this.refresh()));
    this.defer(() => this.cancelRequest());
    void this.refresh();
  }

  private async refresh(): Promise<void> {
    this.cancelRequest();
    const request = this.request = new AbortController();
    const model = this.viewport.textModel;
    try {
      const hints = await this.service.provideInlayHints(this.languageId, TextRange.from(TextPosition.at(0, 0), model.positionAt(model.length)), request.signal);
      if (request.signal.aborted) return;
      this.hints = hints;
      this.render();
    } catch (error) {
      if (!request.signal.aborted) this.onError(error);
    }
  }

  private render(): void {
    for (const element of [...this.viewport.element.querySelectorAll<HTMLElement>(".aster-editor-inlay-hint")]) element.remove();
    const scroll = this.viewport.viewportLayout.scrollPosition;
    for (const hint of this.hints) {
      const element = this.viewport.element.ownerDocument.createElement("span");
      element.className = "aster-editor-inlay-hint";
      element.textContent = typeof hint.label === "string" ? hint.label : hint.label.map(part => part.value).join("");
      const coordinates = this.viewport.getPositionContentCoordinates(hint.position);
      element.style.left = `${coordinates.left - scroll.left + 2}px`;
      element.style.top = `${coordinates.top - scroll.top}px`;
      if (hint.tooltip) element.title = hint.tooltip;
      this.viewport.element.append(element);
    }
  }

  private cancelRequest(): void {
    this.request?.abort();
    this.request = undefined;
  }
}

registerEditorContribution({ id: "editor.contrib.inlayHints", install: context => {
  if (context.kind !== "text" || context.model.largeFile.tooLargeForTokenization) return;
  const service = context.own(context.languageFeaturesService.createInlayHintsService(context.model));
  context.own(new InlayHintsController(context.viewport, service, context.languageId, context.onLanguageError));
} });

import "./media/codelens.css";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";
import { type CodeLensService, type LanguageCodeLens } from "../common/codelens.js";

export type ExecuteCodeLensCommand = (id: string, args: readonly unknown[] | undefined) => void | Promise<void>;

/** Projects provider code lenses as inline command buttons and delegates execution to the host. */
export class CodeLensController extends DisposableOwner {
  private lenses: readonly LanguageCodeLens[] = [];
  private request: AbortController | undefined;

  constructor(private readonly viewport: EditorViewport, private readonly service: CodeLensService, private readonly languageId: string, private readonly onExecuteCommand?: ExecuteCodeLensCommand, private readonly onError: (error: unknown) => void = error => console.error("Aster code lens failed", error)) {
    super();
    this.own(viewport.onDidChangeLayout(() => this.render()));
    this.own(viewport.textModel.onDidChange(() => void this.refresh()));
    this.defer(() => this.request?.abort());
    void this.refresh();
  }

  private async refresh(): Promise<void> {
    this.request?.abort();
    const request = this.request = new AbortController();
    try {
      const lenses = await this.service.provideCodeLenses(this.languageId, request.signal);
      this.lenses = await Promise.all(lenses.map(lens => lens.command || lens.data === undefined
        ? lens
        : this.service.resolveCodeLens(this.languageId, lens, request.signal)));
      if (!request.signal.aborted) this.render();
    } catch (error) {
      if (!request.signal.aborted) this.onError(error);
    }
  }

  private render(): void {
    for (const element of [...this.viewport.element.querySelectorAll<HTMLElement>(".aster-editor-codelens")]) element.remove();
    const scroll = this.viewport.viewportLayout.scrollPosition;
    for (const lens of this.lenses) {
      const command = lens.command;
      if (!command) continue;
      const element = this.viewport.element.ownerDocument.createElement("button");
      element.type = "button";
      element.className = "aster-editor-codelens";
      element.textContent = command.title;
      const coordinates = this.viewport.getPositionContentCoordinates(lens.range.start);
      element.style.left = `${Math.max(4, coordinates.left - scroll.left)}px`;
      element.style.top = `${Math.max(0, coordinates.top - scroll.top - coordinates.height)}px`;
      element.addEventListener("click", () => {
        if (this.onExecuteCommand) {
          try { const result = this.onExecuteCommand(command.id, command.arguments); if (result && typeof (result as Promise<void>).then === "function") void (result as Promise<void>).catch(this.onError); } catch (error) { this.onError(error); }
        } else this.viewport.announceAccessibilityStatus(`Command: ${command.title}`);
      });
      this.viewport.element.append(element);
    }
  }
}

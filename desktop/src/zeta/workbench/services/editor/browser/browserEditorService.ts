import type { IEditorPart } from "../../../browser/parts/editor/editorPart.js";
import type { EditorInput, EditorOpenOptions, IEditorService } from "../common/editorService.js";

/** Projects the Editor Part into the resource-oriented Workbench editor contract. */
export class BrowserEditorService implements IEditorService {
  constructor(private readonly editorPart: IEditorPart) {}

  async openEditor(input: EditorInput, options?: EditorOpenOptions): Promise<void> {
    await this.editorPart.openEditor(input, options);
  }

  focusActiveEditor(): void {
    this.editorPart.focus();
  }
}

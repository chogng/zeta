import { type LanguageWorkspaceSymbol } from "../../../../editor/common/languages/workspaceSymbols.js";
import { type IFileService } from "../../../../platform/files/common/files.js";
import { type IEditorPart } from "../../../browser/parts/editor/editorPart.js";

/** Opens a Workspace Symbol only after a local index result still matches current file content. */
export async function acceptWorkspaceSymbol(symbol: LanguageWorkspaceSymbol, files: IFileService, editor: IEditorPart, quickPick: { hide(): void }, refresh: () => void): Promise<void> {
  const revision = localSymbolRevision(symbol);
  if (revision) {
    try {
      const current = await files.readFile(symbol.resource);
      if (`sha256:${current.revision}` !== revision) {
        refresh();
        return;
      }
    } catch (error) {
      console.error("Could not verify workspace symbol", error);
      refresh();
      return;
    }
  }
  quickPick.hide();
  await editor.openEditor({ resource: symbol.resource }, { selection: symbol.range }).catch(error => console.error("Could not open workspace symbol", error));
}

function localSymbolRevision(symbol: LanguageWorkspaceSymbol): string | undefined {
  if (!symbol.data || typeof symbol.data !== "object") return undefined;
  const data = symbol.data as { source?: unknown; sourceRevision?: unknown };
  return data.source === "localSymbolIndex" && typeof data.sourceRevision === "string" && data.sourceRevision.startsWith("sha256:") ? data.sourceRevision : undefined;
}

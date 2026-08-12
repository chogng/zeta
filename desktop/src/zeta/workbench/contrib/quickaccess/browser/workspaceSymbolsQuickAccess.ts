import { Keybinding, logicalKey } from "../../../../base/common/keybindings.js";
import { DisposableStore } from "../../../../base/common/lifecycle.js";
import { Action2, registerAction2 } from "../../../../platform/actions/common/actions.js";
import { type ServicesAccessor } from "../../../../platform/instantiation/common/instantiation.js";
import { IQuickInputService, type IQuickPickItem } from "../../../../platform/quickinput/common/quickInput.js";
import { type LanguageWorkspaceSymbol } from "../../../../editor/common/languages/workspaceSymbols.js";
import { IEditorPart } from "../../../browser/parts/editor/editorPart.js";
import { ILanguageFeaturesService } from "../../../services/language/common/languageFeaturesService.js";

export const ShowAllSymbolsCommandId = "workbench.action.showAllSymbols";

interface WorkspaceSymbolQuickPickItem extends IQuickPickItem {
  readonly symbol: LanguageWorkspaceSymbol;
}

registerAction2(class ShowAllSymbolsAction extends Action2 {
  constructor() {
    super({
      id: ShowAllSymbolsCommandId,
      title: "Go to Symbol in Workspace",
      f1: true,
      keybinding: { primary: Keybinding.single(logicalKey("t", { primaryKey: true })) },
    });
  }

  override run(accessor: ServicesAccessor): void {
    const service = accessor.get(ILanguageFeaturesService).createWorkspaceSymbolService();
    const editor = accessor.get(IEditorPart);
    const quickPick = accessor.get(IQuickInputService).createQuickPick<WorkspaceSymbolQuickPickItem>();
    const disposables = new DisposableStore();
    let request: AbortController | undefined;
    let requestGeneration = 0;
    disposables.add(service);
    disposables.add(quickPick);
    quickPick.placeholder = "Type the name of a symbol in the workspace";

    const update = (query: string): void => {
      request?.abort();
      const current = request = new AbortController();
      const generation = ++requestGeneration;
      void service.provideWorkspaceSymbols(query, current.signal).then(symbols => {
        if (current.signal.aborted || generation !== requestGeneration) return;
        quickPick.items = symbols.map(symbol => ({
          symbol,
          label: symbol.name,
          description: symbol.containerName,
          detail: `${resourceLabel(symbol.resource)}:${symbol.range.start.lineIndex + 1}`,
        }));
      }).catch(error => {
        if (!current.signal.aborted) console.error("Workspace symbol search failed", error);
      });
    };

    disposables.add(quickPick.onDidChangeValue(update));
    disposables.add(quickPick.onDidAccept(item => {
      quickPick.hide();
      void editor.openEditor({ resource: item.symbol.resource }, { selection: item.symbol.range }).catch(error => console.error("Could not open workspace symbol", error));
    }));
    disposables.add(quickPick.onDidHide(() => { request?.abort(); disposables.dispose(); }));
    quickPick.show();
    update("");
  }
});

function resourceLabel(resource: LanguageWorkspaceSymbol["resource"]): string {
  const path = decodeURIComponent(resource.path);
  return path.slice(path.lastIndexOf("/") + 1) || resource.toString();
}

import * as monaco from "monaco-editor";
import { isDarkColorScheme } from "../../../platform/theme/common/theme.js";
import { IThemeService } from "../../../platform/theme/common/themeService.js";
import { registerEditorPane } from "../../../workbench/browser/parts/editor/editorRegistry.js";
import { registerWorkbenchContribution, WorkbenchPhase } from "../../../workbench/common/contributions.js";
import { MonacoEditorPane } from "../browser/monacoEditorPane.js";
import { MonacoChatInputEditor } from "../browser/monacoChatInputEditor.js";
import { MonacoSyntaxTokenService } from "../browser/monacoSyntaxTokenService.js";
import { MONACO_EDITOR_ID, matchMonacoEditor } from "../common/monacoEditorInput.js";
import { ChatInputEditors } from "../../../workbench/contrib/chat/browser/input/chatInputEditor.js";
import { IRendererApiService } from "../../../workbench/common/services.js";

ChatInputEditors.registerStatic({
  id: "monaco",
  create: (options) => new MonacoChatInputEditor(options),
});

registerEditorPane({
  id: MONACO_EDITOR_ID,
  name: "Code Editor",
  canOpen: matchMonacoEditor,
  create: options => {
    if (!options.textFileService) throw new Error("Monaco Editor requires the Workbench text file service");
    return new MonacoEditorPane(options.textFileService, options.configurationService);
  },
});

registerWorkbenchContribution(
  "workbench.contrib.monacoSyntaxTokens",
  WorkbenchPhase.BlockStartup,
  (accessor) => new MonacoSyntaxTokenService(accessor.get(IRendererApiService)),
);

registerWorkbenchContribution(
  "workbench.contrib.monacoTheme",
  WorkbenchPhase.BlockStartup,
  (accessor) => {
    const themeService = accessor.get(IThemeService);
    const apply = (): void => {
      monaco.editor.setTheme(
        isDarkColorScheme(themeService.getColorTheme().colorScheme)
          ? "vs-dark"
          : "vs",
      );
    };
    apply();
    return themeService.onDidColorThemeChange(apply);
  },
);

import * as monaco from "monaco-editor";
import { isDarkColorScheme } from "../../../platform/theme/common/theme.js";
import { IThemeService } from "../../../platform/theme/common/themeService.js";
import {
  registerEditorPane,
} from "../../../workbench/browser/parts/editor/editorRegistry.js";
import { registerWorkbenchContribution, WorkbenchPhase } from "../../../workbench/common/contributions.js";
import {
  MonacoEditorPane,
} from "../browser/monacoEditorPane.js";
import {
  MONACO_EDITOR_ID,
  matchMonacoEditor,
} from "../common/monacoEditorInput.js";

registerEditorPane({
  id: MONACO_EDITOR_ID,
  name: "Code Editor",
  canOpen: matchMonacoEditor,
  create: (options) => new MonacoEditorPane(
    options.configurationService,
  ),
});

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

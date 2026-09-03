import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';

const desktopDirectory = resolve(import.meta.dirname, '../../../zeta-ts');
const result = spawnSync(process.execPath, [
	'--import',
	'../scripts/zeta-ts/test/ignore-css-imports.ts',
	'--test',
	'--test-concurrency=1',
	...process.argv.slice(2),
	'../.build/desktop/test/src/zeta/base/test/common/event.test.js',
	'../.build/desktop/test/src/zeta/editor/test/common/languageRegistry.test.js',
	'../.build/desktop/test/src/zeta/platform/commands/test/common/commands.test.js',
	'../.build/desktop/test/src/zeta/platform/extensions/test/**/*.test.js',
	'../.build/desktop/test/src/zeta/platform/extensionHost/test/**/*.test.js',
	'../.build/desktop/test/src/zeta/workbench/services/extensions/test/**/*.test.js',
	'../.build/desktop/test/src/zeta/workbench/services/textMate/test/common/textMateGrammarService.test.js',
	'../.build/desktop/test/src/zeta/workbench/services/textMate/test/common/textMateThemeProjection.test.js',
	'../.build/desktop/test/src/zeta/workbench/contrib/codeEditor/test/browser/codeEditorPane.test.js',
	'../.build/desktop/test/src/zeta/workbench/contrib/codeEditor/test/common/editorInput.test.js',
	'../.build/desktop/test/src/zeta/workbench/contrib/preferences/test/browser/settings.test.js',
	'../.build/desktop/test/src/zeta/workbench/services/debug/test/browser/debugService.test.js',
	'../.build/desktop/test/src/zeta/workbench/services/debug/test/common/debugAdapterFactory.test.js',
	'../.build/desktop/test/src/zeta/workbench/services/debug/test/common/launchConfiguration.test.js',
	'../.build/desktop/test/src/zeta/workbench/services/extensionHost/test/**/*.test.js',
	'../.build/desktop/test/src/zeta/workbench/services/language/test/common/languageFeaturesService.test.js',
	'../.build/desktop/test/src/zeta/workbench/services/tasks/test/browser/taskService.test.js',
	'../.build/desktop/test/src/zeta/workbench/services/testing/test/browser/testingService.test.js',
	'../.build/desktop/test/src/zeta/workbench/services/untitled/test/common/untitled-text-editor-service.test.js',
	'../.build/desktop/test/src/zeta/workbench/test/browser/appServerConnectionStateObserver.test.js',
	'../.build/desktop/test/src/zeta/workbench/test/browser/workbench-theme.test.js',
	'../.build/desktop/test/src/zeta/workbench/test/common/theme.test.js',
], {
	cwd: desktopDirectory,
	stdio: 'inherit',
	windowsHide: true,
});

if (result.error) {
	throw result.error;
}
process.exitCode = result.status ?? 1;

import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';

const desktopDirectory = resolve(import.meta.dirname, '../../../zeta-ts');
const result = spawnSync(process.execPath, [
	'--import',
	'../scripts/zeta-ts/test/ignore-css-imports.ts',
	'--import',
	'../scripts/zeta-ts/test/editor-test-environment.ts',
	'--test',
	'--test-concurrency=1',
	...process.argv.slice(2),
	'../.build/desktop/test/src/zeta/editor/**/test/**/*.test.js',
	'../.build/desktop/test/src/zeta/workbench/contrib/academic/**/test/**/*.test.js',
	'../.build/desktop/test/src/zeta/workbench/contrib/codeEditor/**/test/**/*.test.js',
	'../.build/desktop/test/src/zeta/workbench/contrib/multiDiffEditor/**/test/**/*.test.js',
	'../.build/desktop/test/src/zeta/workbench/contrib/bulkEdit/**/test/**/*.test.js',
	'../.build/desktop/test/src/zeta/workbench/contrib/documentEditor/**/test/**/*.test.js',
	'../.build/desktop/test/src/zeta/workbench/services/documentCollaboration/**/test/**/*.test.js',
	'../.build/desktop/test/src/zeta/workbench/services/language/**/test/**/*.test.js',
	'../.build/desktop/test/src/zeta/workbench/services/textMate/**/test/**/*.test.js',
	'../.build/desktop/test/src/zeta/workbench/services/textfile/**/test/**/*.test.js',
	'../.build/desktop/test/src/zeta/workbench/services/workingCopy/**/test/**/*.test.js',
], {
	cwd: desktopDirectory,
	stdio: 'inherit',
});

if (result.error) {
	throw result.error;
}
process.exitCode = result.status ?? 1;

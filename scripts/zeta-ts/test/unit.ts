import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';

const desktopDirectory = resolve(import.meta.dirname, '../../../zeta-ts');
const result = spawnSync(process.execPath, [
	'--import',
	'../scripts/zeta-ts/test/ignore-css-imports.ts',
	'--test',
	'--test-concurrency=1',
	...process.argv.slice(2),
	'../.build/desktop/test/src/zeta/**/test/**/*.test.js',
	'../.build/desktop/test/test/architecture/*.test.js',
], {
	cwd: desktopDirectory,
	stdio: 'inherit',
	windowsHide: true,
});

if (result.error) {
	throw result.error;
}
process.exitCode = result.status ?? 1;

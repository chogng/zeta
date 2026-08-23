import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';

const repositoryRoot = resolve(import.meta.dirname, '../..');

export function runPnpmScript(project: string, script: string, argumentsList: readonly string[]): void {
	const corepackExecutable = process.platform === 'win32' ? 'corepack.cmd' : 'corepack';
	const result = spawnSync(corepackExecutable, [
		'pnpm',
		'--dir',
		resolve(repositoryRoot, project),
		'run',
		script,
		...argumentsList,
	], {
		cwd: repositoryRoot,
		stdio: 'inherit',
		windowsHide: true,
	});

	if (result.error) {
		throw result.error;
	}
	process.exitCode = result.status ?? 1;
}

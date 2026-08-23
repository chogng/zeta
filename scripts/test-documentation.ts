import { runPnpmScript } from './test/pnpm-script.ts';

runPnpmScript('docs-site', 'test', process.argv.slice(2));

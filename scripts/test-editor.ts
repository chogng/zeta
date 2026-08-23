import { runPnpmScript } from './test/pnpm-script.ts';

runPnpmScript('zeta-ts', 'test:editor', process.argv.slice(2));

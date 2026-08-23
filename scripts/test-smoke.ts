import { runPnpmScript } from './test/pnpm-script.ts';

runPnpmScript('zeta-ts', 'test:smoke:desktop', process.argv.slice(2));

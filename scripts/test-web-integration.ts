import { runPnpmScript } from './test/pnpm-script.ts';

runPnpmScript('zeta-ts', 'test:smoke:browser:full', process.argv.slice(2));

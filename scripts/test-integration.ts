import { runPnpmScript } from './test/pnpm-script.ts';

runPnpmScript('zeta-ts', 'test:editor:browser', process.argv.slice(2));

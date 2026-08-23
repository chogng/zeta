import { runPnpmScript } from './test/pnpm-script.ts';

runPnpmScript('zeta-ts', 'test:main', process.argv.slice(2));

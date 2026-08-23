import { resolve } from 'node:path';

export function developmentArtifactsPath(appPath: string, ...segments: readonly string[]): string {
	return resolve(appPath, '..', '.build', 'desktop', ...segments);
}

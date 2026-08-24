import { ConfigurationsRegistry } from '../../../../platform/configuration/common/configurationRegistry.js';

export type ScmWorkingSetDefault = 'current' | 'empty';
export type ScmDiffDecorations = 'all' | 'gutter' | 'overview' | 'minimap' | 'none';
export type ScmDiffDecorationsGutterAction = 'diff' | 'none';

export const ScmConfiguration = Object.freeze({
	workingSetsEnabled: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: 'scm.workingSets.enabled',
		defaultValue: false,
		parse(value: unknown): boolean {
			if (typeof value !== 'boolean') throw new TypeError('scm.workingSets.enabled must be a boolean');
			return value;
		},
	}),
	workingSetsDefault: ConfigurationsRegistry.registerConfiguration<ScmWorkingSetDefault>({
		key: 'scm.workingSets.default',
		defaultValue: 'current',
		parse(value: unknown): ScmWorkingSetDefault {
			if (value !== 'current' && value !== 'empty') throw new TypeError('scm.workingSets.default must be current or empty');
			return value;
		},
	}),
	diffDecorations: ConfigurationsRegistry.registerConfiguration<ScmDiffDecorations>({
		key: 'scm.diffDecorations',
		defaultValue: 'all',
		parse(value: unknown): ScmDiffDecorations {
			if (value !== 'all' && value !== 'gutter' && value !== 'overview' && value !== 'minimap' && value !== 'none') {
				throw new TypeError('scm.diffDecorations must be all, gutter, overview, minimap, or none');
			}
			return value;
		},
	}),
	diffDecorationsGutterAction: ConfigurationsRegistry.registerConfiguration<ScmDiffDecorationsGutterAction>({
		key: 'scm.diffDecorationsGutterAction',
		defaultValue: 'diff',
		parse(value: unknown): ScmDiffDecorationsGutterAction {
			if (value !== 'diff' && value !== 'none') throw new TypeError('scm.diffDecorationsGutterAction must be diff or none');
			return value;
		},
	}),
});

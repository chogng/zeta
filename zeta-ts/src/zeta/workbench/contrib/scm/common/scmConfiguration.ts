import { ConfigurationsRegistry } from '../../../../platform/configuration/common/configurationRegistry.js';

export type ScmWorkingSetDefault = 'current' | 'empty';

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
});

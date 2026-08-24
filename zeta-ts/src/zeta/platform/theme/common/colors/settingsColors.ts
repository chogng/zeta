import { registerColor } from '../colorRegistry.js';

const owner = 'workbench.preferences';

export const itemBackground = registerColor('settings.itemBackground', { dark: '#252526', light: '#f3f3f3' }, {
	description: 'Background for configuration items in the Settings editor.',
	owner,
});

export const itemSeparator = registerColor('settings.itemSeparator', { dark: '#383838', light: '#e0e0e0' }, {
	description: 'Separator between configuration items in the Settings editor.',
	owner,
});

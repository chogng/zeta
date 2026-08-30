import { ConfigurationsRegistry } from '../../../platform/configuration/common/configurationRegistry.js';
import { EDITOR_MODEL_DEFAULTS } from '../core/misc/textModelDefaults.js';

function parseBoolean(value: unknown, key: string): boolean {
	if (typeof value !== 'boolean') throw new TypeError(`${key} must be boolean`);
	return value;
}

function parseInteger(value: unknown, key: string): number {
	if (!Number.isSafeInteger(value) || (value as number) < 1 || (value as number) > 100) {
		throw new RangeError(`${key} must be an integer between 1 and 100`);
	}
	return value as number;
}

/** Configuration addresses consumed while creating and maintaining text models. */
export const EditorModelConfiguration = Object.freeze({
	tabSize: ConfigurationsRegistry.registerConfiguration<number>({
		key: 'editor.tabSize',
		defaultValue: EDITOR_MODEL_DEFAULTS.tabSize,
		parse: value => parseInteger(value, 'editor.tabSize'),
		setting: { title: 'Tab size', description: 'Set the number of columns represented by one tab.', valueType: 'number', minimum: 1, maximum: 100 },
	}),
	indentSize: ConfigurationsRegistry.registerConfiguration<number | 'tabSize'>({
		key: 'editor.indentSize',
		defaultValue: 'tabSize',
		parse(value: unknown): number | 'tabSize' {
			return value === 'tabSize' ? value : parseInteger(value, 'editor.indentSize');
		},
	}),
	insertSpaces: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: 'editor.insertSpaces',
		defaultValue: EDITOR_MODEL_DEFAULTS.insertSpaces,
		parse: value => parseBoolean(value, 'editor.insertSpaces'),
	}),
	detectIndentation: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: 'editor.detectIndentation',
		defaultValue: EDITOR_MODEL_DEFAULTS.detectIndentation,
		parse: value => parseBoolean(value, 'editor.detectIndentation'),
	}),
	trimAutoWhitespace: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: 'editor.trimAutoWhitespace',
		defaultValue: EDITOR_MODEL_DEFAULTS.trimAutoWhitespace,
		parse: value => parseBoolean(value, 'editor.trimAutoWhitespace'),
	}),
	largeFileOptimizations: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: 'editor.largeFileOptimizations',
		defaultValue: EDITOR_MODEL_DEFAULTS.largeFileOptimizations,
		parse: value => parseBoolean(value, 'editor.largeFileOptimizations'),
	}),
	bracketPairColorizationEnabled: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: 'editor.bracketPairColorization.enabled',
		defaultValue: EDITOR_MODEL_DEFAULTS.bracketPairColorizationOptions.enabled,
		parse: value => parseBoolean(value, 'editor.bracketPairColorization.enabled'),
		setting: { title: 'Bracket pair colorization', description: 'Use matching colors to distinguish nested bracket pairs.', valueType: 'boolean' },
	}),
	bracketPairColorizationIndependentColorPool: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: 'editor.bracketPairColorization.independentColorPoolPerBracketType',
		defaultValue: EDITOR_MODEL_DEFAULTS.bracketPairColorizationOptions.independentColorPoolPerBracketType,
		parse: value => parseBoolean(value, 'editor.bracketPairColorization.independentColorPoolPerBracketType'),
	}),
	filesEol: ConfigurationsRegistry.registerConfiguration<'auto' | '\n' | '\r\n'>({
		key: 'files.eol',
		defaultValue: 'auto',
		parse(value: unknown): 'auto' | '\n' | '\r\n' {
			if (value === 'auto' || value === '\n' || value === '\r\n') return value;
			throw new TypeError('files.eol must be auto, LF, or CRLF');
		},
	}),
	restoreUndoStack: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: 'files.restoreUndoStack',
		defaultValue: true,
		parse: value => parseBoolean(value, 'files.restoreUndoStack'),
	}),
});

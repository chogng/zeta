import { EditorLineWrapping } from '../../common/config/editorOptions.js';
import { isObject, type Mutable } from '../../../base/common/types.js';

type MutableOptions = Mutable<Readonly<Record<string, unknown>>>;

/** Converts legacy browser-editor option shapes before the current contract is validated. */
export function migrateOptions<T extends object>(options: T): T {
	if (!isObject(options)) {
		throw new TypeError('Editor options must be an object');
	}

	const migrated = { ...options } as MutableOptions;
	migrateAlias(migrated, 'wordWrap', 'lineWrapping', value => booleanOrNamed(value, EditorLineWrapping.On, EditorLineWrapping.Off));
	migrateAlias(migrated, 'showLineNumbers', 'lineNumbers', value => booleanOrNamed(value, 'on', 'off'));
	migrateBooleanOption(migrated, 'lineNumbers', 'on', 'off');
	migrateAlias(migrated, 'activeLineHighlight', 'renderLineHighlight', migrateRenderLineHighlight);
	migrateAlias(migrated, 'renderIndentGuides', 'guides', value => ({ indentation: booleanValue(value) }));
	migrateBooleanOption(migrated, 'renderWhitespace', 'boundary', 'none');
	migrateBooleanOption(migrated, 'matchBrackets', 'always', 'never');
	migrateBooleanOption(migrated, 'occurrencesHighlight', 'singleFile', 'off');
	migrateBooleanOption(migrated, 'defaultColorDecorators', 'auto', 'never');
	return migrated as T;
}

function migrateAlias(options: MutableOptions, sourceKey: string, targetKey: string, convert: (value: unknown) => unknown): void {
	if (!(sourceKey in options)) {
		return;
	}
	if (!(targetKey in options)) {
		options[targetKey] = convert(options[sourceKey]);
	}
	delete options[sourceKey];
}

function migrateBooleanOption(options: MutableOptions, key: string, enabledValue: unknown, disabledValue: unknown): void {
	if (typeof options[key] === 'boolean') {
		options[key] = options[key] ? enabledValue : disabledValue;
	}
}

function booleanOrNamed(value: unknown, enabledValue: unknown, disabledValue: unknown): unknown {
	if (value === true || value === 'on') {
		return enabledValue;
	}
	if (value === false || value === 'off') {
		return disabledValue;
	}
	throw new TypeError('Legacy editor option must be boolean, on, or off');
}

function booleanValue(value: unknown): boolean {
	if (typeof value !== 'boolean') {
		throw new TypeError('Legacy editor option must be boolean');
	}
	return value;
}

function migrateRenderLineHighlight(value: unknown): 'none' | 'line' {
	if (value === false || value === 'off' || value === 'none') {
		return 'none';
	}
	if (value === true || value === 'on' || value === 'line') {
		return 'line';
	}
	throw new TypeError('Legacy active-line highlight option is invalid');
}

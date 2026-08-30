import { type IEditorOptions } from '../../common/config/editorOptions.js';

export interface ISettingsReader {
	(key: string): unknown;
}

export interface ISettingsWriter {
	(key: string, value: unknown): void;
}

export class EditorSettingMigration {
	public static items: EditorSettingMigration[] = [];

	constructor(
		public readonly key: string,
		public readonly migrate: (value: unknown, read: ISettingsReader, write: ISettingsWriter) => void,
	) { }

	apply(options: unknown): void {
		const value = EditorSettingMigration._read(options, this.key);
		const read = (key: string) => EditorSettingMigration._read(options, key);
		const write = (key: string, nextValue: unknown) => EditorSettingMigration._write(options, key, nextValue);
		this.migrate(value, read, write);
	}

	private static _read(source: unknown, key: string): unknown {
		if (source === undefined || source === null) return undefined;
		const firstDotIndex = key.indexOf('.');
		if (firstDotIndex >= 0) {
			const firstSegment = key.substring(0, firstDotIndex);
			return this._read((source as Record<string, unknown>)[firstSegment], key.substring(firstDotIndex + 1));
		}
		return (source as Record<string, unknown>)[key];
	}

	private static _write(target: unknown, key: string, value: unknown): void {
		const firstDotIndex = key.indexOf('.');
		if (firstDotIndex >= 0) {
			const firstSegment = key.substring(0, firstDotIndex);
			const record = target as Record<string, unknown>;
			record[firstSegment] = record[firstSegment] || {};
			this._write(record[firstSegment], key.substring(firstDotIndex + 1), value);
			return;
		}
		(target as Record<string, unknown>)[key] = value;
	}
}

function registerEditorSettingMigration(key: string, migrate: (value: unknown, read: ISettingsReader, write: ISettingsWriter) => void): void {
	EditorSettingMigration.items.push(new EditorSettingMigration(key, migrate));
}

function registerSimpleEditorSettingMigration(key: string, values: [unknown, unknown][]): void {
	registerEditorSettingMigration(key, (value, _read, write) => {
		if (value === undefined) return;
		for (const [oldValue, newValue] of values) {
			if (value === oldValue) {
				write(key, newValue);
				return;
			}
		}
	});
}

export function migrateOptions(options: IEditorOptions): void {
	EditorSettingMigration.items.forEach(migration => migration.apply(options));
}

registerSimpleEditorSettingMigration('wordWrap', [[true, 'on'], [false, 'off']]);
registerSimpleEditorSettingMigration('lineNumbers', [[true, 'on'], [false, 'off']]);
registerSimpleEditorSettingMigration('cursorBlinking', [['visible', 'solid']]);
registerSimpleEditorSettingMigration('renderWhitespace', [[true, 'boundary'], [false, 'none']]);
registerSimpleEditorSettingMigration('renderLineHighlight', [[true, 'line'], [false, 'none']]);
registerSimpleEditorSettingMigration('acceptSuggestionOnEnter', [[true, 'on'], [false, 'off']]);
registerSimpleEditorSettingMigration('tabCompletion', [[false, 'off'], [true, 'onlySnippets']]);
registerSimpleEditorSettingMigration('hover', [[true, { enabled: true }], [false, { enabled: false }]]);
registerSimpleEditorSettingMigration('parameterHints', [[true, { enabled: true }], [false, { enabled: false }]]);
registerSimpleEditorSettingMigration('autoIndent', [[false, 'advanced'], [true, 'full']]);
registerSimpleEditorSettingMigration('matchBrackets', [[true, 'always'], [false, 'never']]);
registerSimpleEditorSettingMigration('renderFinalNewline', [[true, 'on'], [false, 'off']]);
registerSimpleEditorSettingMigration('cursorSmoothCaretAnimation', [[true, 'on'], [false, 'off']]);
registerSimpleEditorSettingMigration('occurrencesHighlight', [[true, 'singleFile'], [false, 'off']]);
registerSimpleEditorSettingMigration('defaultColorDecorators', [[true, 'auto'], [false, 'never']]);
registerSimpleEditorSettingMigration('minimap.autohide', [[true, 'mouseover'], [false, 'none']]);

registerEditorSettingMigration('autoClosingBrackets', (value, read, write) => {
	if (value !== false) return;
	write('autoClosingBrackets', 'never');
	if (read('autoClosingQuotes') === undefined) write('autoClosingQuotes', 'never');
	if (read('autoSurround') === undefined) write('autoSurround', 'never');
});

registerEditorSettingMigration('quickSuggestions', (value, _read, write) => {
	if (typeof value !== 'boolean') return;
	const nextValue = value ? 'on' : 'off';
	write('quickSuggestions', { comments: nextValue, strings: nextValue, other: nextValue });
});

registerEditorSettingMigration('renderIndentGuides', (value, read, write) => {
	if (value === undefined) return;
	write('renderIndentGuides', undefined);
	if (read('guides.indentation') === undefined) write('guides.indentation', Boolean(value));
});

registerEditorSettingMigration('highlightActiveIndentGuide', (value, read, write) => {
	if (value === undefined) return;
	write('highlightActiveIndentGuide', undefined);
	if (read('guides.highlightActiveIndentation') === undefined) write('guides.highlightActiveIndentation', Boolean(value));
});

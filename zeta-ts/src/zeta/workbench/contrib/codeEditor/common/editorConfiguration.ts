import { ConfigurationsRegistry } from "../../../../platform/configuration/common/configurationRegistry.js";
import { EditorIndentationKind } from "../../../../editor/common/editorIndentation.js";
import { EditorLineWrapping } from "../../../../editor/browser/viewModel/visualLineProjection.js";

/** Typed user preferences owned by the Workbench code-editor integration. */
export const CodeEditorConfiguration = Object.freeze({
	fontFamily: ConfigurationsRegistry.registerConfiguration<string>({
		key: "editor.fontFamily",
		defaultValue: "",
		parse(value: unknown): string {
			if (typeof value === "string" && value.length <= 256 && !/[\r\n\0]/u.test(value)) return value;
			throw new TypeError("editor.fontFamily must be a single-line string no longer than 256 characters");
		},
	}),
	fontSize: ConfigurationsRegistry.registerConfiguration<number>({
		key: "editor.fontSize",
		defaultValue: 13,
		parse(value: unknown): number {
			if (Number.isSafeInteger(value) && (value as number) >= 8 && (value as number) <= 40) return value as number;
			throw new RangeError(`editor.fontSize must be an integer between 8 and 40; received ${String(value)}`);
		},
	}),
	lineHeight: ConfigurationsRegistry.registerConfiguration<number>({
		key: "editor.lineHeight",
		defaultValue: 20,
		parse(value: unknown): number {
			if (Number.isSafeInteger(value) && (value as number) >= 12 && (value as number) <= 80) return value as number;
			throw new RangeError(`editor.lineHeight must be an integer between 12 and 80; received ${String(value)}`);
		},
	}),
	fontLigatures: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "editor.fontLigatures",
		defaultValue: false,
		parse: value => parseBoolean(value, "editor.fontLigatures"),
	}),
	wordWrap: ConfigurationsRegistry.registerConfiguration<EditorLineWrapping>({
		key: "editor.wordWrap",
		defaultValue: EditorLineWrapping.Off,
		parse(value: unknown): EditorLineWrapping {
			if (value === EditorLineWrapping.Off || value === EditorLineWrapping.On) return value;
			throw new TypeError(`editor.wordWrap must be off or on; received ${String(value)}`);
		},
	}),
	minimapEnabled: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "editor.minimap.enabled",
		defaultValue: true,
		parse: value => parseBoolean(value, "editor.minimap.enabled"),
	}),
	lineNumbers: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "editor.lineNumbers",
		defaultValue: true,
		parse: value => parseBoolean(value, "editor.lineNumbers"),
	}),
	indentationGuides: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "editor.guides.indentation",
		defaultValue: true,
		parse: value => parseBoolean(value, "editor.guides.indentation"),
	}),
	bracketPairColorization: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "editor.bracketPairColorization.enabled",
		defaultValue: true,
		parse: value => parseBoolean(value, "editor.bracketPairColorization.enabled"),
	}),
	stickyScroll: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "editor.stickyScroll.enabled",
		defaultValue: true,
		parse: value => parseBoolean(value, "editor.stickyScroll.enabled"),
	}),
	highlightActiveLine: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "editor.highlightActiveLine",
		defaultValue: true,
		parse: value => parseBoolean(value, "editor.highlightActiveLine"),
	}),
	unicodeHighlights: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "editor.unicodeHighlights",
		defaultValue: true,
		parse: value => parseBoolean(value, "editor.unicodeHighlights"),
	}),
	indentationKind: ConfigurationsRegistry.registerConfiguration<EditorIndentationKind>({
		key: "editor.indentation",
		defaultValue: EditorIndentationKind.Spaces,
		parse(value: unknown): EditorIndentationKind {
			if (value === EditorIndentationKind.Spaces || value === EditorIndentationKind.Tabs) return value;
			throw new TypeError(`editor.indentation must be spaces or tabs; received ${String(value)}`);
		},
	}),
	tabSize: ConfigurationsRegistry.registerConfiguration<number>({
		key: "editor.tabSize",
		defaultValue: 4,
		parse(value: unknown): number {
			if (Number.isSafeInteger(value) && (value as number) >= 1 && (value as number) <= 32) return value as number;
			throw new RangeError(`editor.tabSize must be an integer between 1 and 32; received ${String(value)}`);
		},
	}),
	formatOnSave: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "editor.formatOnSave",
		defaultValue: false,
		parse: value => parseBoolean(value, "editor.formatOnSave"),
	}),
	findSeedFromSelection: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "editor.find.seedSearchStringFromSelection",
		defaultValue: true,
		parse: value => parseBoolean(value, "editor.find.seedSearchStringFromSelection"),
	}),
	findAutoFindInSelection: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "editor.find.autoFindInSelection",
		defaultValue: false,
		parse: value => parseBoolean(value, "editor.find.autoFindInSelection"),
	}),
	findLoop: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "editor.find.loop",
		defaultValue: true,
		parse: value => parseBoolean(value, "editor.find.loop"),
	}),
	findMatchCase: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "editor.find.matchCase",
		defaultValue: false,
		parse: value => parseBoolean(value, "editor.find.matchCase"),
	}),
	findWholeWord: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "editor.find.wholeWord",
		defaultValue: false,
		parse: value => parseBoolean(value, "editor.find.wholeWord"),
	}),
	findRegularExpression: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "editor.find.useRegularExpression",
		defaultValue: false,
		parse: value => parseBoolean(value, "editor.find.useRegularExpression"),
	}),
	suggestions: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "editor.suggest.enabled",
		defaultValue: true,
		parse: value => parseBoolean(value, "editor.suggest.enabled"),
	}),
	inlineCompletions: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "editor.inlineSuggest.enabled",
		defaultValue: true,
		parse: value => parseBoolean(value, "editor.inlineSuggest.enabled"),
	}),
	parameterHints: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "editor.parameterHints.enabled",
		defaultValue: true,
		parse: value => parseBoolean(value, "editor.parameterHints.enabled"),
	}),
	inlayHints: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "editor.inlayHints.enabled",
		defaultValue: true,
		parse: value => parseBoolean(value, "editor.inlayHints.enabled"),
	}),
	codeLens: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "editor.codeLens",
		defaultValue: true,
		parse: value => parseBoolean(value, "editor.codeLens"),
	}),
	diffShowLineNumbers: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "diffEditor.showLineNumbers",
		defaultValue: true,
		parse: value => parseBoolean(value, "diffEditor.showLineNumbers"),
	}),
	diffShowInlineChanges: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "diffEditor.showInlineChanges",
		defaultValue: true,
		parse: value => parseBoolean(value, "diffEditor.showInlineChanges"),
	}),
	diffLoopChanges: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "diffEditor.loopChanges",
		defaultValue: true,
		parse: value => parseBoolean(value, "diffEditor.loopChanges"),
	}),
	diffBreadcrumbs: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "diffEditor.breadcrumbs.enabled",
		defaultValue: true,
		parse: value => parseBoolean(value, "diffEditor.breadcrumbs.enabled"),
	}),
	insertFinalNewLine: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "files.insertFinalNewline",
		defaultValue: false,
		parse: value => parseBoolean(value, "files.insertFinalNewline"),
	}),
});

function parseBoolean(value: unknown, key: string): boolean {
	if (typeof value === "boolean") return value;
	throw new TypeError(`${key} must be a boolean; received ${String(value)}`);
}

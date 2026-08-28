import { ConfigurationsRegistry } from "../../../../platform/configuration/common/configurationRegistry.js";
import { EditorIndentationKind } from "../../../../editor/common/editorIndentation.js";
import { EditorLineWrapping } from "../../../../editor/common/config/editorOptions.js";

export type WrappingIndentSetting = "none" | "same" | "indent" | "deepIndent";
export type MatchBracketsSetting = "never" | "near" | "always";

/** Typed user preferences owned by the Workbench code-editor integration. */
export const CodeEditorConfiguration = Object.freeze({
	fontFamily: ConfigurationsRegistry.registerConfiguration<string>({
		key: "editor.fontFamily",
		defaultValue: "",
		parse(value: unknown): string {
			if (typeof value === "string" && value.length <= 256 && !/[\r\n\0]/u.test(value)) return value;
			throw new TypeError("editor.fontFamily must be a single-line string no longer than 256 characters");
		},
		setting: textSetting("Font family", "Use a CSS font-family list, or leave this empty to use the default monospace font.", "Default monospace"),
	}),
	fontSize: ConfigurationsRegistry.registerConfiguration<number>({
		key: "editor.fontSize",
		defaultValue: 13,
		parse(value: unknown): number {
			if (Number.isSafeInteger(value) && (value as number) >= 8 && (value as number) <= 40) return value as number;
			throw new RangeError(`editor.fontSize must be an integer between 8 and 40; received ${String(value)}`);
		},
		setting: numberSetting("Font size", "Set the editor text size in pixels.", 8, 40),
	}),
	lineHeight: ConfigurationsRegistry.registerConfiguration<number>({
		key: "editor.lineHeight",
		defaultValue: 20,
		parse(value: unknown): number {
			if (Number.isSafeInteger(value) && (value as number) >= 12 && (value as number) <= 80) return value as number;
			throw new RangeError(`editor.lineHeight must be an integer between 12 and 80; received ${String(value)}`);
		},
		setting: numberSetting("Line height", "Set the height of each editor line in pixels.", 12, 80),
	}),
	fontLigatures: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "editor.fontLigatures",
		defaultValue: false,
		parse: value => parseBoolean(value, "editor.fontLigatures"),
		setting: booleanSetting("Font ligatures", "Use programming ligatures when the selected font supports them."),
	}),
	experimentalGpuAcceleration: ConfigurationsRegistry.registerConfiguration<"on" | "off">({
		key: "editor.experimentalGpuAcceleration",
		defaultValue: "off",
		parse(value: unknown): "on" | "off" {
			if (value === "on" || value === "off") return value;
			throw new TypeError(`editor.experimentalGpuAcceleration must be on or off; received ${String(value)}`);
		},
		setting: selectSetting("GPU acceleration", "Draw eligible visible editor text through the experimental WebGPU backend.", [
			{ value: "off", label: "Off" },
			{ value: "on", label: "On" },
		]),
	}),
	wordWrap: ConfigurationsRegistry.registerConfiguration<EditorLineWrapping>({
		key: "editor.wordWrap",
		defaultValue: EditorLineWrapping.Off,
		parse(value: unknown): EditorLineWrapping {
			if (value === EditorLineWrapping.Off || value === EditorLineWrapping.On) return value;
			throw new TypeError(`editor.wordWrap must be off or on; received ${String(value)}`);
		},
		setting: selectSetting("Word wrap", "Wrap long lines at the editor viewport instead of scrolling horizontally.", [
			{ value: EditorLineWrapping.Off, label: "Off" },
			{ value: EditorLineWrapping.On, label: "On" },
		]),
	}),
	wrappingIndent: ConfigurationsRegistry.registerConfiguration<WrappingIndentSetting>({
		key: "editor.wrappingIndent",
		defaultValue: "same",
		parse(value: unknown): WrappingIndentSetting {
			if (value === "none" || value === "same" || value === "indent" || value === "deepIndent") return value;
			throw new TypeError(`editor.wrappingIndent must be none, same, indent, or deepIndent; received ${String(value)}`);
		},
		setting: selectSetting("Wrapping indent", "Choose the indentation applied to continuation rows created by word wrapping.", [
			{ value: "none", label: "None" },
			{ value: "same", label: "Same" },
			{ value: "indent", label: "Indent" },
			{ value: "deepIndent", label: "Deep indent" },
		]),
	}),
	minimapEnabled: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "editor.minimap.enabled",
		defaultValue: true,
		parse: value => parseBoolean(value, "editor.minimap.enabled"),
		setting: booleanSetting("Enabled", "Show a compact document overview on the right side of the editor."),
	}),
	lineNumbers: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "editor.lineNumbers",
		defaultValue: true,
		parse: value => parseBoolean(value, "editor.lineNumbers"),
		setting: booleanSetting("Line numbers", "Show line numbers in the editor gutter."),
	}),
	indentationGuides: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "editor.guides.indentation",
		defaultValue: true,
		parse: value => parseBoolean(value, "editor.guides.indentation"),
		setting: booleanSetting("Indentation guides", "Show vertical guides aligned with indentation levels."),
	}),
	bracketPairColorization: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "editor.bracketPairColorization.enabled",
		defaultValue: true,
		parse: value => parseBoolean(value, "editor.bracketPairColorization.enabled"),
		setting: booleanSetting("Bracket pair colorization", "Use matching colors to distinguish nested bracket pairs."),
	}),
	matchBrackets: ConfigurationsRegistry.registerConfiguration<MatchBracketsSetting>({
		key: "editor.matchBrackets",
		defaultValue: "always",
		parse(value: unknown): MatchBracketsSetting {
			if (value === "never" || value === "near" || value === "always") return value;
			throw new TypeError(`editor.matchBrackets must be never, near, or always; received ${String(value)}`);
		},
		setting: selectSetting("Match brackets", "Choose when matching bracket pairs are highlighted.", [
			{ value: "never", label: "Never" },
			{ value: "near", label: "Near cursor" },
			{ value: "always", label: "Always" },
		]),
	}),
	stickyScroll: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "editor.stickyScroll.enabled",
		defaultValue: true,
		parse: value => parseBoolean(value, "editor.stickyScroll.enabled"),
		setting: booleanSetting("Sticky scroll", "Keep enclosing scopes visible at the top while scrolling."),
	}),
	highlightActiveLine: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "editor.highlightActiveLine",
		defaultValue: true,
		parse: value => parseBoolean(value, "editor.highlightActiveLine"),
		setting: booleanSetting("Highlight active line", "Give the line containing the cursor a subtle background highlight."),
	}),
	unicodeHighlights: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "editor.unicodeHighlights",
		defaultValue: true,
		parse: value => parseBoolean(value, "editor.unicodeHighlights"),
		setting: booleanSetting("Unicode highlights", "Call attention to invisible or easily confused Unicode characters."),
	}),
	indentationKind: ConfigurationsRegistry.registerConfiguration<EditorIndentationKind>({
		key: "editor.indentation",
		defaultValue: EditorIndentationKind.Spaces,
		parse(value: unknown): EditorIndentationKind {
			if (value === EditorIndentationKind.Spaces || value === EditorIndentationKind.Tabs) return value;
			throw new TypeError(`editor.indentation must be spaces or tabs; received ${String(value)}`);
		},
		setting: selectSetting("Indent using", "Choose whether indentation inserts spaces or tab characters.", [
			{ value: EditorIndentationKind.Spaces, label: "Spaces" },
			{ value: EditorIndentationKind.Tabs, label: "Tabs" },
		]),
	}),
	tabSize: ConfigurationsRegistry.registerConfiguration<number>({
		key: "editor.tabSize",
		defaultValue: 4,
		parse(value: unknown): number {
			if (Number.isSafeInteger(value) && (value as number) >= 1 && (value as number) <= 32) return value as number;
			throw new RangeError(`editor.tabSize must be an integer between 1 and 32; received ${String(value)}`);
		},
		setting: numberSetting("Tab size", "Set the number of columns represented by one indentation level.", 1, 32),
	}),
	formatOnSave: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "editor.formatOnSave",
		defaultValue: false,
		parse: value => parseBoolean(value, "editor.formatOnSave"),
		setting: booleanSetting("Format on save", "Run the active language formatter before saving a file."),
	}),
	findSeedFromSelection: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "editor.find.seedSearchStringFromSelection",
		defaultValue: true,
		parse: value => parseBoolean(value, "editor.find.seedSearchStringFromSelection"),
		setting: booleanSetting("Seed from selection", "Use a single-line selection as the initial Find query."),
	}),
	findAutoFindInSelection: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "editor.find.autoFindInSelection",
		defaultValue: false,
		parse: value => parseBoolean(value, "editor.find.autoFindInSelection"),
		setting: booleanSetting("Find in selection automatically", "Limit Find to the current non-empty selection when the widget opens."),
	}),
	findLoop: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "editor.find.loop",
		defaultValue: true,
		parse: value => parseBoolean(value, "editor.find.loop"),
		setting: booleanSetting("Loop through matches", "Wrap from the final match to the first match and back again."),
	}),
	findMatchCase: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "editor.find.matchCase",
		defaultValue: false,
		parse: value => parseBoolean(value, "editor.find.matchCase"),
		setting: booleanSetting("Match case by default", "Open Find with case-sensitive matching enabled."),
	}),
	findWholeWord: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "editor.find.wholeWord",
		defaultValue: false,
		parse: value => parseBoolean(value, "editor.find.wholeWord"),
		setting: booleanSetting("Whole word by default", "Open Find with whole-word matching enabled."),
	}),
	findRegularExpression: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "editor.find.useRegularExpression",
		defaultValue: false,
		parse: value => parseBoolean(value, "editor.find.useRegularExpression"),
		setting: booleanSetting("Regular expression by default", "Open Find with regular-expression matching enabled."),
	}),
	suggestions: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "editor.suggest.enabled",
		defaultValue: true,
		parse: value => parseBoolean(value, "editor.suggest.enabled"),
		setting: booleanSetting("Suggestions", "Show completion suggestions from language providers."),
	}),
	inlineCompletions: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "editor.inlineSuggest.enabled",
		defaultValue: true,
		parse: value => parseBoolean(value, "editor.inlineSuggest.enabled"),
		setting: booleanSetting("Inline completions", "Show provider-supplied completion text directly in the editor."),
	}),
	parameterHints: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "editor.parameterHints.enabled",
		defaultValue: true,
		parse: value => parseBoolean(value, "editor.parameterHints.enabled"),
		setting: booleanSetting("Parameter hints", "Show signature information while entering function arguments."),
	}),
	inlayHints: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "editor.inlayHints.enabled",
		defaultValue: true,
		parse: value => parseBoolean(value, "editor.inlayHints.enabled"),
		setting: booleanSetting("Inlay hints", "Show inferred types, parameter names, and other inline annotations."),
	}),
	codeLens: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "editor.codeLens",
		defaultValue: true,
		parse: value => parseBoolean(value, "editor.codeLens"),
		setting: booleanSetting("CodeLens", "Show provider actions and references near relevant code."),
	}),
	diffShowLineNumbers: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "diffEditor.showLineNumbers",
		defaultValue: true,
		parse: value => parseBoolean(value, "diffEditor.showLineNumbers"),
		setting: booleanSetting("Line numbers", "Show original and modified line numbers in diff cells."),
	}),
	diffShowInlineChanges: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "diffEditor.showInlineChanges",
		defaultValue: true,
		parse: value => parseBoolean(value, "diffEditor.showInlineChanges"),
		setting: booleanSetting("Inline change highlights", "Highlight the exact changed ranges within modified lines."),
	}),
	diffLoopChanges: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "diffEditor.loopChanges",
		defaultValue: true,
		parse: value => parseBoolean(value, "diffEditor.loopChanges"),
		setting: booleanSetting("Loop through changes", "Wrap change navigation from the final difference to the first."),
	}),
	diffBreadcrumbs: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "diffEditor.breadcrumbs.enabled",
		defaultValue: true,
		parse: value => parseBoolean(value, "diffEditor.breadcrumbs.enabled"),
		setting: booleanSetting("Change breadcrumbs", "Show the current change position while navigating a diff."),
	}),
	insertFinalNewLine: ConfigurationsRegistry.registerConfiguration<boolean>({
		key: "files.insertFinalNewline",
		defaultValue: false,
		parse: value => parseBoolean(value, "files.insertFinalNewline"),
		setting: booleanSetting("Insert final newline", "Ensure non-empty files end with a line feed when saved."),
	}),
});

function booleanSetting(title: string, description: string) {
	return { valueType: "boolean", title, description } as const;
}

function numberSetting(title: string, description: string, minimum: number, maximum: number) {
	return { valueType: "number", title, description, minimum, maximum } as const;
}

function selectSetting<T extends string>(title: string, description: string, options: readonly { readonly value: T; readonly label: string }[]) {
	return { valueType: "select", title, description, options } as const;
}

function textSetting(title: string, description: string, placeholder: string) {
	return { valueType: "text", title, description, placeholder } as const;
}

function parseBoolean(value: unknown, key: string): boolean {
	if (typeof value === "boolean") return value;
	throw new TypeError(`${key} must be a boolean; received ${String(value)}`);
}

import './media/editorConfiguration.css';
import type { IContextMenuProvider } from '../../../../base/browser/contextmenu.js';
import type { IContextViewProvider } from '../../../../base/browser/ui/contextview/contextview.js';
import type { IClipboardService } from '../../../../platform/clipboard/common/clipboardService.js';
import type { IConfigurationService } from '../../../../platform/configuration/common/configurationService.js';
import { EditorLineWrapping } from '../../../../editor/browser/viewModel/visualLineProjection.js';
import { EditorIndentationKind } from '../../../../editor/common/editorIndentation.js';
import { EditorSelectionConfiguration } from '../../../common/editorSelectionConfiguration.js';
import { CodeEditorConfiguration } from '../../codeEditor/common/editorConfiguration.js';
import { WorkspaceSearchConfiguration } from '../../search/common/searchConfiguration.js';
import { booleanSetting, type ConfigurationSettingsGroupDescriptor, informationSetting, numberSetting, selectSetting, textSetting } from '../common/settingsDescriptors.js';
import { ConfigurationItemsContribution } from './configurationItems.js';

interface EditorConfigurationContributionOptions {
	readonly clipboardService: IClipboardService;
	readonly configurationService: IConfigurationService;
	readonly contextMenuProvider: IContextMenuProvider;
	readonly contextViewProvider: IContextViewProvider;
	readonly onStatus: (message: string, isError: boolean) => void;
}

/** Product settings declarations for Stanza-backed code editors. */
export class EditorConfigurationContribution extends ConfigurationItemsContribution {
	constructor(document: Document, options: EditorConfigurationContributionOptions) {
		super('editor', document, {
			clipboardService: options.clipboardService,
			configurationService: options.configurationService,
			contextMenuProvider: options.contextMenuProvider,
			contextViewProvider: options.contextViewProvider,
			groups: EditorConfigurationGroups,
			onStatus: options.onStatus,
			presentation: 'editor',
		});
	}
}

const EditorConfigurationGroups: readonly ConfigurationSettingsGroupDescriptor[] = [
	{
		id: 'selection',
		title: 'Editor selection',
		description: 'Choose what new documents start with while existing resources continue to resolve by content type.',
		settings: [
			selectSetting(EditorSelectionConfiguration.defaultNewDocumentEditor, 'Default editor for new documents', 'Follow the active build mode, or explicitly prefer the Code or Academic editor for new untitled documents.', [
				{ value: 'buildMode', label: 'Default' },
				{ value: 'code', label: 'Code' },
				{ value: 'academic', label: 'Academic' },
			]),
			informationSetting('editor.info.existingResources', 'Existing resources', 'Source files continue to open in the Code Editor, while Academic content types and .zeta-paper files open in the Structured Editor.', 'Automatic'),
			informationSetting('editor.info.associations', 'Editor associations', 'Custom glob-to-editor associations are not persisted yet. Use the resource type resolver until the association service is added.', 'Not available'),
		],
	},
	{
		id: 'typography',
		title: 'Typography',
		description: 'Set the typeface and size used for code.',
		settings: [
			textSetting(CodeEditorConfiguration.fontFamily, 'Font family', 'Use a CSS font-family list, or leave this empty to use the default monospace font.', 'Default monospace'),
			numberSetting(CodeEditorConfiguration.fontSize, 'Font size', 'Set the editor text size in pixels.', 8, 40),
			numberSetting(CodeEditorConfiguration.lineHeight, 'Line height', 'Set the height of each editor line in pixels.', 12, 80),
			booleanSetting(CodeEditorConfiguration.fontLigatures, 'Font ligatures', 'Use programming ligatures when the selected font supports them.'),
		],
	},
	{
		id: 'display',
		title: 'Display',
		description: 'Control how code is presented while you work.',
		settings: [
			selectSetting(CodeEditorConfiguration.wordWrap, 'Word wrap', 'Wrap long lines at the editor viewport instead of scrolling horizontally.', [
				{ value: EditorLineWrapping.Off, label: 'Off' },
				{ value: EditorLineWrapping.On, label: 'On' },
			]),
			booleanSetting(CodeEditorConfiguration.lineNumbers, 'Line numbers', 'Show line numbers in the editor gutter.'),
			booleanSetting(CodeEditorConfiguration.indentationGuides, 'Indentation guides', 'Show vertical guides aligned with indentation levels.'),
			booleanSetting(CodeEditorConfiguration.bracketPairColorization, 'Bracket pair colorization', 'Use matching colors to distinguish nested bracket pairs.'),
			booleanSetting(CodeEditorConfiguration.stickyScroll, 'Sticky scroll', 'Keep enclosing scopes visible at the top while scrolling.'),
			booleanSetting(CodeEditorConfiguration.highlightActiveLine, 'Highlight active line', 'Give the line containing the cursor a subtle background highlight.'),
			booleanSetting(CodeEditorConfiguration.unicodeHighlights, 'Unicode highlights', 'Call attention to invisible or easily confused Unicode characters.'),
		],
	},
	{
		id: 'minimap',
		title: 'Minimap',
		description: 'Control the compact document overview shown beside the editor.',
		settings: [
			booleanSetting(CodeEditorConfiguration.minimapEnabled, 'Enabled', 'Show a compact document overview on the right side of the editor.'),
		],
	},
	{
		id: 'editing',
		title: 'Editing',
		description: 'Choose default editing and formatting behavior.',
		settings: [
			selectSetting(CodeEditorConfiguration.indentationKind, 'Indent using', 'Choose whether indentation inserts spaces or tab characters.', [
				{ value: EditorIndentationKind.Spaces, label: 'Spaces' },
				{ value: EditorIndentationKind.Tabs, label: 'Tabs' },
			]),
			numberSetting(CodeEditorConfiguration.tabSize, 'Tab size', 'Set the number of columns represented by one indentation level.', 1, 32),
			booleanSetting(CodeEditorConfiguration.formatOnSave, 'Format on save', 'Run the active language formatter before saving a file.'),
		],
	},
	{
		id: 'code-intelligence',
		title: 'Code intelligence',
		description: 'Control language-aware assistance inside code editors.',
		settings: [
			booleanSetting(CodeEditorConfiguration.suggestions, 'Suggestions', 'Show completion suggestions from language providers.'),
			booleanSetting(CodeEditorConfiguration.inlineCompletions, 'Inline completions', 'Show provider-supplied completion text directly in the editor.'),
			booleanSetting(CodeEditorConfiguration.parameterHints, 'Parameter hints', 'Show signature information while entering function arguments.'),
			booleanSetting(CodeEditorConfiguration.inlayHints, 'Inlay hints', 'Show inferred types, parameter names, and other inline annotations.'),
			booleanSetting(CodeEditorConfiguration.codeLens, 'CodeLens', 'Show provider actions and references near relevant code.'),
		],
	},
	{
		id: 'find-and-replace',
		title: 'Find and replace',
		description: 'Choose how the editor-local Find widget starts and navigates matches.',
		settings: [
			booleanSetting(CodeEditorConfiguration.findSeedFromSelection, 'Seed from selection', 'Use a single-line selection as the initial Find query.'),
			booleanSetting(CodeEditorConfiguration.findAutoFindInSelection, 'Find in selection automatically', 'Limit Find to the current non-empty selection when the widget opens.'),
			booleanSetting(CodeEditorConfiguration.findLoop, 'Loop through matches', 'Wrap from the final match to the first match and back again.'),
			booleanSetting(CodeEditorConfiguration.findMatchCase, 'Match case by default', 'Open Find with case-sensitive matching enabled.'),
			booleanSetting(CodeEditorConfiguration.findWholeWord, 'Whole word by default', 'Open Find with whole-word matching enabled.'),
			booleanSetting(CodeEditorConfiguration.findRegularExpression, 'Regular expression by default', 'Open Find with regular-expression matching enabled.'),
		],
	},
	{
		id: 'workspace-search',
		title: 'Workspace search',
		description: 'Set defaults for searching files across the current workspace.',
		settings: [
			booleanSetting(WorkspaceSearchConfiguration.matchCase, 'Match case', 'Start workspace searches in case-sensitive mode.'),
			booleanSetting(WorkspaceSearchConfiguration.smartCase, 'Smart case', 'Use case-sensitive matching automatically when the query contains uppercase characters.'),
			booleanSetting(WorkspaceSearchConfiguration.regularExpression, 'Use regular expressions', 'Interpret workspace search queries as regular expressions by default.'),
			textSetting(WorkspaceSearchConfiguration.includePatterns, 'Files to include', 'Comma-separated glob patterns included in new workspace searches.', 'src/**, packages/**'),
			textSetting(WorkspaceSearchConfiguration.excludePatterns, 'Files to exclude', 'Comma-separated glob patterns excluded from new workspace searches.', '**/node_modules/**, **/dist/**'),
			numberSetting(WorkspaceSearchConfiguration.maxResults, 'Maximum results', 'Stop a workspace search after this many matches.', 100, 5_000),
		],
	},
	{
		id: 'diff-editor',
		title: 'Diff editor',
		description: 'Control side-by-side comparison presentation and navigation.',
		settings: [
			booleanSetting(CodeEditorConfiguration.diffShowLineNumbers, 'Line numbers', 'Show original and modified line numbers in diff cells.'),
			booleanSetting(CodeEditorConfiguration.diffShowInlineChanges, 'Inline change highlights', 'Highlight the exact changed ranges within modified lines.'),
			booleanSetting(CodeEditorConfiguration.diffLoopChanges, 'Loop through changes', 'Wrap change navigation from the final difference to the first.'),
			booleanSetting(CodeEditorConfiguration.diffBreadcrumbs, 'Change breadcrumbs', 'Show the current change position while navigating a diff.'),
		],
	},
	{
		id: 'files',
		title: 'Files',
		description: 'Apply small consistency fixes when saving code files.',
		settings: [
			booleanSetting(CodeEditorConfiguration.insertFinalNewLine, 'Insert final newline', 'Ensure non-empty files end with a line feed when saved.'),
		],
	},
];

import './media/placeholderText.css';
import { wrapInReloadableClass1 } from '../../../../platform/observable/common/wrapInReloadableClass.js';
import { EditorContributionInstantiation } from '../../../browser/editorExtensions.js';
import { registerCodeEditorContribution } from '../../../browser/widget/codeEditor/codeEditorContributions.js';
import { WidgetPlaceholderTextContribution } from './placeholderTextContribution.js';

registerCodeEditorContribution({
	id: WidgetPlaceholderTextContribution.ID,
	instantiation: EditorContributionInstantiation.Eager,
	descriptor: wrapInReloadableClass1(() => WidgetPlaceholderTextContribution),
});

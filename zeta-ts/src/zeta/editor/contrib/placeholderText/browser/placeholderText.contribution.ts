import './placeholderText.css';
import { wrapInReloadableClass1 } from '../../../../platform/observable/common/wrapInReloadableClass.js';
import { EditorContributionInstantiation } from '../../../browser/editorExtensions.js';
import { registerCodeEditorContribution } from '../../../browser/widget/codeEditor/codeEditorContributions.js';
import { PlaceholderTextContribution } from './placeholderTextContribution.js';

registerCodeEditorContribution({
	id: PlaceholderTextContribution.ID,
	instantiation: EditorContributionInstantiation.Eager,
	descriptor: wrapInReloadableClass1(() => PlaceholderTextContribution),
});

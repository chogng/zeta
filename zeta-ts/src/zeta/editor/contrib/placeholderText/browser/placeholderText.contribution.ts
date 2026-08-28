import './media/placeholderText.css';
import { wrapInReloadableClass1 } from '../../../../platform/observable/common/wrapInReloadableClass.js';
import { CodeEditorContributionInstantiation, registerCodeEditorContribution } from '../../../browser/widget/codeEditor/codeEditorContributions.js';
import { PlaceholderTextContribution } from './placeholderTextContribution.js';

registerCodeEditorContribution({
	id: PlaceholderTextContribution.ID,
	instantiation: CodeEditorContributionInstantiation.Eager,
	descriptor: wrapInReloadableClass1(() => PlaceholderTextContribution),
});

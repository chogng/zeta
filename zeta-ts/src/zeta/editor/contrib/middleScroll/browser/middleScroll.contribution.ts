import { ServiceConstructionDescriptor } from '../../../../platform/instantiation/common/instantiation.js';
import { EditorContributionInstantiation, registerTextEditorCapabilityContribution } from '../../../browser/editorExtensions.js';
import { MiddleScrollController } from './middleScrollController.js';

registerTextEditorCapabilityContribution({
	id: MiddleScrollController.ID,
	runtime: {
		descriptor: new ServiceConstructionDescriptor(MiddleScrollController),
		instantiation: EditorContributionInstantiation.BeforeFirstInteraction,
	},
});

import { BrowserClipboardService } from '../../../../platform/clipboard/browser/browserClipboardService.js';
import { Disposable } from '../../../../base/common/lifecycle.js';
import { IClipboardService } from '../../../../platform/clipboard/common/clipboardService.js';
import { IInstantiationService, ServiceConstructionDescriptor, type IInstantiationService as InstantiationService } from '../../../../platform/instantiation/common/instantiation.js';
import { EditorContributionInstantiation, registerTextEditorCapabilityContribution, type TextEditorContributionContext } from '../../../browser/editorExtensions.js';
import { TextEditorCapability } from '../../textEditorCapabilities.js';
import { ClipboardController } from './clipboardController.js';

class ClipboardContribution extends Disposable {
	public constructor(context: TextEditorContributionContext, instantiationService: InstantiationService) {
		super();
		const ownerWindow = context.view.element.ownerDocument.defaultView;
		const clipboardService = instantiationService.getOptional(IClipboardService)
			?? new BrowserClipboardService(ownerWindow?.navigator.clipboard);
		this._register(new ClipboardController(context.view.editContext, context.viewport, context.selections, clipboardService, {
			semanticTokens: context.getOptionalCapability(TextEditorCapability.semanticTokenSource),
			isEditingAllowed: () => !context.view.compositionController.composing,
		}));
	}
}

registerTextEditorCapabilityContribution({
	id: 'editor.contrib.clipboard',
	runtime: {
		descriptor: new ServiceConstructionDescriptor(ClipboardContribution, { serviceDependencies: [IInstantiationService] }),
		instantiation: EditorContributionInstantiation.Eager,
	},
});

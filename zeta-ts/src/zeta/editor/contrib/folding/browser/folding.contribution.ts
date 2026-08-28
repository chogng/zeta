import { EditorContributionInstantiation, registerEditorContribution } from "../../../browser/editorExtensions.js";
import { toDisposable } from "../../../../base/common/lifecycle.js";
import { FoldingController } from "./folding.js";
import { TextEditorCapability } from "../../textEditorCapabilities.js";
import { EditorFoldingModel } from "./foldingModel.js";
import { EditorHiddenRangeModel } from "./hiddenRangeModel.js";
import { computeEditorIndentFoldingRanges } from "./indentRangeProvider.js";
import { computeEditorLanguageFoldingRanges, mergeEditorFoldingRanges } from "./syntaxRangeProvider.js";
import { FoldingDecorationProvider } from "./foldingDecorations.js";
import { SyncDescriptor } from "../../../../platform/instantiation/common/instantiation.js";
import { FoldingRangeService } from '../common/folding.js';

registerEditorContribution({
	id: "editor.contrib.folding",
	configure: context => {
		const folding = context.register(new EditorFoldingModel(context.model));
		const largeFile = context.model.largeFile.tooLargeForTokenization;
		const hiddenRanges = largeFile ? undefined : context.register(new EditorHiddenRangeModel(context.model, folding));
		const languageFolding = largeFile ? undefined : context.register(new FoldingRangeService(context.model, context.languageFeaturesService.foldingRangeProvider, context.options.input.resource));
		let serverRanges: readonly { readonly startLineIndex: number; readonly endLineIndex: number }[] = [];
		let requestSerial = 0;
		let requestController: AbortController | undefined;
		let disposed = false;
		context.register(toDisposable(() => {
			disposed = true;
			requestSerial += 1;
			requestController?.abort();
			requestController = undefined;
		}));
		const update = () => {
			if (disposed) return;
			folding.setProviderRanges(largeFile ? [] : mergeEditorFoldingRanges(serverRanges, computeEditorLanguageFoldingRanges(context.model, context.languageId, context.configurations), computeEditorIndentFoldingRanges(context.model)));
		};
		const refresh = () => {
			if (disposed) return;
			update();
			if (!languageFolding) return;
			requestController?.abort();
			const controller = new AbortController();
			requestController = controller;
			const serial = ++requestSerial;
			void languageFolding.provideFoldingRanges(context.languageId, controller.signal).then(ranges => {
				if (disposed || controller.signal.aborted || serial !== requestSerial) return;
				serverRanges = ranges;
				update();
			}, error => {
				if (!disposed && !controller.signal.aborted && serial === requestSerial) context.onLanguageError(error);
			});
		};
		refresh();
		if (!largeFile) context.register(context.model.onDidChange(refresh));
		context.provideCapability(TextEditorCapability.folding, folding);
		if (hiddenRanges) {
			context.setLineProjection({ visibilitySource: hiddenRanges });
			context.addDecorationSource(context.register(new FoldingDecorationProvider(folding)));
		}
	},
	runtime: {
		descriptor: new SyncDescriptor(FoldingController),
		instantiation: EditorContributionInstantiation.Eager,
	},
});

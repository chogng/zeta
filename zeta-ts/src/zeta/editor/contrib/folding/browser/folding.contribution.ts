import { registerEditorContribution } from "../../../browser/editorExtensions.js";
import { toDisposable } from "../../../../base/common/lifecycle.js";
import { FoldingController } from "./folding.js";
import { TextEditorCapability } from "../../textEditorCapabilities.js";
import { EditorFoldingModel } from "./foldingModel.js";
import { EditorHiddenRangeModel } from "./hiddenRangeModel.js";
import { computeEditorIndentFoldingRanges } from "./indentRangeProvider.js";
import { computeEditorLanguageFoldingRanges, mergeEditorFoldingRanges } from "./syntaxRangeProvider.js";
import { FoldingDecorationProvider } from "./foldingDecorations.js";
import { RustSyntaxFoldingService } from "../../../browser/services/rustSyntaxFoldingService.js";

registerEditorContribution({
	id: "editor.contrib.folding",
	configure: context => {
		const folding = context.register(new EditorFoldingModel(context.model));
		const largeFile = context.model.largeFile.tooLargeForTokenization;
		const hiddenRanges = largeFile ? undefined : context.register(new EditorHiddenRangeModel(context.model, folding));
		const rustSyntaxFacts = largeFile ? undefined : context.getOptionalCapability(TextEditorCapability.rustSyntaxFacts);
		const languageFolding = largeFile ? undefined : context.register(context.languageFeaturesService.createFoldingRangeService(context.model, context.options.input.resource));
		let syntaxFolding: RustSyntaxFoldingService | undefined;
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
			folding.setProviderRanges(largeFile ? [] : mergeEditorFoldingRanges(serverRanges, syntaxFolding?.ranges ?? [], computeEditorLanguageFoldingRanges(context.model, context.languageId, context.configurations), computeEditorIndentFoldingRanges(context.model)));
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
		if (rustSyntaxFacts) syntaxFolding = context.register(new RustSyntaxFoldingService(context.model, context.languageId, rustSyntaxFacts, update, context.onLanguageError));
		refresh();
		if (!largeFile) context.register(context.model.onDidChange(refresh));
		context.provideCapability(TextEditorCapability.folding, folding);
		if (hiddenRanges) {
			context.setLineProjection({ visibilitySource: hiddenRanges });
			context.addLineGutterDecoration(new FoldingDecorationProvider(folding));
		}
	},
	install: context => {
		if (context.kind !== "text" || context.model.largeFile.tooLargeForTokenization) return;
		context.register(new FoldingController(context.view.element, context.viewport, context.selections, context.getCapability(TextEditorCapability.folding)));
	},
});

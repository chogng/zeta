import { ISymbolIndexApi } from "../../../../platform/symbolIndex/common/symbolIndexApi.js";
import { ISymbolIndexService } from "../../../../platform/symbolIndex/common/symbolIndexService.js";
import { IWorkspaceContextService } from "../../../../platform/workspace/common/workspace.js";
import { registerWorkbenchServiceContribution } from "../../../browser/workbenchServiceContributions.js";
import { ILanguageFeaturesService } from "../../language/common/languageFeaturesService.js";
import { AppServerSymbolIndexService } from "./appServerSymbolIndexService.js";
import { registerSymbolIndexWorkspaceSymbolProvider } from "./symbolIndexWorkspaceSymbolProvider.js";

registerWorkbenchServiceContribution({
	service: ISymbolIndexService,
	dependencies: [ISymbolIndexApi, ILanguageFeaturesService, IWorkspaceContextService],
	install: context => {
		const service = new AppServerSymbolIndexService(context.container.get(ISymbolIndexApi));
		context.register(registerSymbolIndexWorkspaceSymbolProvider(context.container.get(ILanguageFeaturesService), service, context.container.get(IWorkspaceContextService)));
		return service;
	},
});

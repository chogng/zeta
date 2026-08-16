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
    const service = new AppServerSymbolIndexService(context.services.get(ISymbolIndexApi));
    context.own(registerSymbolIndexWorkspaceSymbolProvider(context.services.get(ILanguageFeaturesService), service, context.services.get(IWorkspaceContextService)));
    return service;
  },
});

import { registerWorkbenchServiceContribution } from "../../../browser/workbenchServiceContributions.js";
import { ISymbolIndexService } from "../../../../platform/symbolIndex/common/symbolIndexService.js";
import { ILanguageFeaturesService } from "../../../services/language/common/languageFeaturesService.js";
import { AppServerSymbolIndexService } from "../../../services/symbolIndex/browser/appServerSymbolIndexService.js";
import { registerSymbolIndexWorkspaceSymbolProvider } from "../../../services/symbolIndex/browser/symbolIndexWorkspaceSymbolProvider.js";

registerWorkbenchServiceContribution(context => {
  const service = new AppServerSymbolIndexService(context.rendererHost.symbolIndex);
  context.services.set(ISymbolIndexService, service);
  context.own(registerSymbolIndexWorkspaceSymbolProvider(context.services.get(ILanguageFeaturesService), service, context.workspaceContext));
});

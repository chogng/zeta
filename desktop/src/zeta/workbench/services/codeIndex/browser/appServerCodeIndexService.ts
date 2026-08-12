import type { ProviderConfigDto, SemanticCodeIndexAutomaticContextDto, SemanticCodeIndexSelectionDto } from "../../../../../../generated/app-server/types.js";
import type { ICodeIndexApi } from "../../../../platform/codeIndex/common/codeIndexApi.js";
import type { ICodeIndexService } from "../../../../platform/codeIndex/common/codeIndexService.js";

export class AppServerCodeIndexService implements ICodeIndexService {
  constructor(private readonly api: ICodeIndexApi) {}

  readConfig() { return this.api.readConfig(); }

  configureProvider(config: ProviderConfigDto, expectedRevision: number) {
    return this.api.configureProvider({ commandId: commandId("provider"), expectedRevision, config });
  }

  configure(selection: SemanticCodeIndexSelectionDto, automaticContext: SemanticCodeIndexAutomaticContextDto, expectedRevision: number) {
    return this.api.configure({ commandId: commandId("configure"), expectedRevision, selection, automaticContext });
  }

  authorize(expectedRevision: number) {
    return this.api.authorize({ commandId: commandId("authorize"), expectedRevision });
  }

  revoke(expectedRevision: number) {
    return this.api.revoke({ commandId: commandId("revoke"), expectedRevision });
  }

  status() { return this.api.status(); }

  cancel() { return this.api.cancel(); }

  retry() { return this.api.retry(); }
}

function commandId(operation: string): string {
  return `desktop-code-index-${operation}-${crypto.randomUUID()}`;
}

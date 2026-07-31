import type { SyntaxAnalysisSnapshotDto, SyntaxChangeParams, SyntaxCloseParams, SyntaxOpenParams } from "../../../../../generated/app-server/types.js";
import type { ISyntaxAnalysisService, SyntaxAnalysisSnapshot, SyntaxDocumentChangeRequest, SyntaxDocumentCloseRequest, SyntaxDocumentOpenRequest } from "../common/syntaxAnalysisService.js";

/** Narrow App Server capability consumed by the syntax-service adapter. */
export interface ISyntaxAnalysisApi {
  open(params: SyntaxOpenParams): Promise<SyntaxAnalysisSnapshotDto>;
  change(params: SyntaxChangeParams): Promise<SyntaxAnalysisSnapshotDto>;
  close(params: SyntaxCloseParams): Promise<void>;
}

/** Exposes the App Server syntax capability through the renderer service contract. */
export class AppServerSyntaxAnalysisService implements ISyntaxAnalysisService {
  constructor(private readonly api: ISyntaxAnalysisApi) {}

  open(request: SyntaxDocumentOpenRequest): Promise<SyntaxAnalysisSnapshot> {
    return this.api.open({ ...request });
  }

  change(request: SyntaxDocumentChangeRequest): Promise<SyntaxAnalysisSnapshot> {
    return this.api.change({ ...request, edits: request.edits.map(edit => ({ ...edit })) });
  }

  close(request: SyntaxDocumentCloseRequest): Promise<void> {
    return this.api.close({ ...request });
  }
}

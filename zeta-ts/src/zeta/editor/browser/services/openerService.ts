import { type URI } from '../../../base/common/uri.js';
import { type IOpenerService as IPlatformOpenerService } from '../../../platform/opener/common/openerService.js';
import { type ICodeEditorService } from './codeEditorService.js';

/** Opens editor resources first and delegates external HTTP(S) targets to the host opener. */
export class EditorOpenerService {
	constructor(private readonly codeEditors: ICodeEditorService, private readonly external: IPlatformOpenerService) {}

	public async open(resource: URI): Promise<boolean> {
		if (await this.codeEditors.openCodeEditor(resource)) {
			return true;
		}
		await this.external.openExternal(resource.toString());
		return true;
	}
}

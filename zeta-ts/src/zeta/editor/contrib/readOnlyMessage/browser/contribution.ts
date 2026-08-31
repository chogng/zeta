import { MarkdownString } from '../../../../base/common/htmlContent.js';
import { Disposable } from '../../../../base/common/lifecycle.js';
import * as nls from '../../../../nls.js';
import { ServiceConstructionDescriptor } from '../../../../platform/instantiation/common/instantiation.js';
import { EditorContributionInstantiation, registerTextEditorCapabilityContribution, type TextEditorContributionContext } from '../../../browser/editorExtensions.js';
import type { IEditorContribution } from '../../../common/editorCommon.js';
import { MessageController } from '../../message/browser/messageController.js';

/** Turns rejected read-only edits into an editor-positioned explanation. */
export class ReadOnlyMessageController extends Disposable implements IEditorContribution {
	public static readonly ID = 'editor.contrib.readOnlyMessageController';

	constructor(private readonly context: TextEditorContributionContext) {
		super();
		this._register(context.editor.onDidAttemptReadOnlyEdit(() => this.showMessage()));
	}

	private showMessage(): void {
		const controller = MessageController.get(this.context.editor);
		const position = this.context.editor.getPosition();
		if (!controller || !position) return;
		const message = this.context.options.readOnlyMessage
			?? new MarkdownString(nls.localize('editor.readonly', 'Cannot edit in read-only editor'));
		controller.showMessage(message, position);
	}
}

registerTextEditorCapabilityContribution({
	id: ReadOnlyMessageController.ID,
	runtime: {
		descriptor: new ServiceConstructionDescriptor(ReadOnlyMessageController),
		instantiation: EditorContributionInstantiation.BeforeFirstInteraction,
	},
});

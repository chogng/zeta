import { noEvent } from '../../../base/common/event.js';
import type { IEditorService } from '../../services/editor/common/editorService.js';

/** Empty observable editor state for tests that only exercise editor commands. */
export const emptyEditorServiceState = Object.freeze({
	onDidActiveEditorChange: noEvent,
	onDidVisibleEditorsChange: noEvent,
	activeEditor: undefined,
	visibleEditors: Object.freeze([]),
}) satisfies Pick<IEditorService, 'onDidActiveEditorChange' | 'onDidVisibleEditorsChange' | 'activeEditor' | 'visibleEditors'>;

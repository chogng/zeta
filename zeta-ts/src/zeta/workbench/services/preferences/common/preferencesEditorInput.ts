import { URI } from '../../../../base/common/uri.js';
import type { EditorInput } from '../../editor/common/editorService.js';

export const PreferencesEditorContentType = 'application/vnd.zeta.preferences';
export const PreferencesEditorResource = URI.parse('zeta-preferences:/preferences');

/** Creates the singleton input routed to the Workbench Preferences editor. */
export function createPreferencesEditorInput(): EditorInput {
	return {
		resource: PreferencesEditorResource,
		contentType: PreferencesEditorContentType,
		label: 'Zeta Settings',
		readOnly: true,
	};
}

export function isPreferencesEditorInput(input: EditorInput): boolean {
	return input.contentType === PreferencesEditorContentType || input.resource.toString() === PreferencesEditorResource.toString();
}

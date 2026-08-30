import * as objects from '../../../../base/common/objects.js';
import { ICodeEditor } from '../../editorBrowser.js';
import { ICodeEditorService } from '../../services/codeEditorService.js';
import { CodeEditorWidget, ICodeEditorWidgetOptions } from './codeEditorWidget.js';
import { ConfigurationChangedEvent, IEditorOptions } from '../../../common/config/editorOptions.js';
import { ILanguageConfigurationService } from '../../../common/languages/languageConfigurationRegistry.js';
import { ILanguageFeaturesService } from '../../../common/services/languageFeatures.js';
import { IAccessibilityService } from '../../../../platform/accessibility/common/accessibility.js';
import { ICommandService } from '../../../../platform/commands/common/commands.js';
import { IContextKeyService } from '../../../../platform/contextkey/common/contextkey.js';
import { IInstantiationService, ServicesAccessor } from '../../../../platform/instantiation/common/instantiation.js';
import { INotificationService } from '../../../../platform/notification/common/notification.js';
import { IThemeService } from '../../../../platform/theme/common/themeService.js';
import { IUserInteractionService } from '../../../../platform/userInteraction/browser/userInteractionService.js';

export class EmbeddedCodeEditorWidget extends CodeEditorWidget {
	private readonly overwriteOptions: IEditorOptions;

	constructor(
		domElement: HTMLElement,
		options: IEditorOptions,
		codeEditorWidgetOptions: ICodeEditorWidgetOptions,
		private readonly parentEditor: ICodeEditor,
		@IInstantiationService instantiationService: IInstantiationService,
		@ICodeEditorService codeEditorService: ICodeEditorService,
		@ICommandService commandService: ICommandService,
		@IContextKeyService contextKeyService: IContextKeyService,
		@IThemeService themeService: IThemeService,
		@INotificationService notificationService: INotificationService,
		@IAccessibilityService accessibilityService: IAccessibilityService,
		@ILanguageConfigurationService languageConfigurationService: ILanguageConfigurationService,
		@ILanguageFeaturesService languageFeaturesService: ILanguageFeaturesService,
		@IUserInteractionService userInteractionService: IUserInteractionService,
	) {
		super(domElement, { ...parentEditor.getRawOptions(), overflowWidgetsDomNode: parentEditor.getOverflowWidgetsDomNode() }, codeEditorWidgetOptions, instantiationService, codeEditorService, commandService, contextKeyService, themeService, notificationService, accessibilityService, languageConfigurationService, languageFeaturesService, userInteractionService);

		this.overwriteOptions = { ...options };
		super.updateOptions(this.overwriteOptions);
		this._register(parentEditor.onDidChangeConfiguration((event: ConfigurationChangedEvent) => this.onParentConfigurationChanged(event)));
	}

	getParentEditor(): ICodeEditor {
		return this.parentEditor;
	}

	private onParentConfigurationChanged(_event: ConfigurationChangedEvent): void {
		super.updateOptions(this.parentEditor.getRawOptions());
		super.updateOptions(this.overwriteOptions);
	}

	override updateOptions(newOptions: IEditorOptions): void {
		objects.mixin(this.overwriteOptions, newOptions, true);
		super.updateOptions(this.overwriteOptions);
	}
}

export function getOuterEditor(accessor: ServicesAccessor): ICodeEditor | null {
	const editor = accessor.get(ICodeEditorService).getFocusedCodeEditor();
	return editor instanceof EmbeddedCodeEditorWidget ? editor.getParentEditor() : editor;
}

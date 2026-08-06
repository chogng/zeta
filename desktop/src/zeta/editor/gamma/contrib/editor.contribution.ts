import { registerEditorPane } from "../../../workbench/browser/parts/editor/editorRegistry.js";
import { AlphaEmbeddedTextEditorFactory } from "../../alpha/browser/alphaEmbeddedTextEditor.js";
import { academicProfile } from "../academic/browser/profile.js";
import { DocumentEditorPane } from "../browser/documentEditorPane.js";
import { createDocumentEditorPaneOptions, findDocumentEditorProfile, matchDocumentEditorProfiles } from "../browser/profile.js";

const profiles = [academicProfile] as const;

for (const profile of profiles) {
  registerEditorPane({
    id: profile.editorId,
    name: profile.editorName,
    canOpen: input => matchDocumentEditorProfiles(input, [profile]),
    create: options => {
      if (!options.textFileService) throw new Error("Structured Editor requires the Workbench text file service");
      if (!options.input) throw new Error("Structured Editor requires its Workbench input during construction");
      const selectedProfile = findDocumentEditorProfile(options.input, [profile]);
      if (!selectedProfile) throw new Error("Structured Editor has no profile for " + options.input.resource.toString());
      const paneOptions = createDocumentEditorPaneOptions(selectedProfile, {
        onSave: options.onSave,
        workingCopyService: options.workingCopyService,
        embeddedTextEditorFactory: options.embeddedTextEditorFactory ?? new AlphaEmbeddedTextEditorFactory({
          textMateService: options.textMateService,
          languageFeaturesService: options.languageFeaturesService,
        }),
      });
      return new DocumentEditorPane(options.textFileService, paneOptions);
    },
  });
}

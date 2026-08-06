import { registerEditorPane } from "../../../../../workbench/browser/parts/editor/editorRegistry.js";
import { AlphaEmbeddedTextEditorFactory } from "../../../../alpha/browser/alphaEmbeddedTextEditor.js";
import { GamaEditorPane } from "../../../browser/gamaEditorPane.js";
import { createGamaEditorPaneOptions, findGamaEditorProfile, matchGamaEditorProfiles } from "../../../browser/services/gamaEditorProfile.js";
import { academicProfile } from "./profile.js";

const profiles = [academicProfile] as const;

for (const profile of profiles) {
  registerEditorPane({
    id: profile.editorId,
    name: profile.editorName,
    canOpen: input => matchGamaEditorProfiles(input, [profile]),
    create: options => {
      if (!options.textFileService) throw new Error("Gama editor requires the Workbench text file service");
      if (!options.input) throw new Error("Gama editor requires its Workbench input during construction");
      const selectedProfile = findGamaEditorProfile(options.input, [profile]);
      if (!selectedProfile) throw new Error("Gama editor has no profile for " + options.input.resource.toString());
      const paneOptions = createGamaEditorPaneOptions(selectedProfile, {
        onSave: options.onSave,
        workingCopyService: options.workingCopyService,
        embeddedTextEditorFactory: options.embeddedTextEditorFactory ?? new AlphaEmbeddedTextEditorFactory({
          textMateService: options.textMateService,
          languageFeaturesService: options.languageFeaturesService,
        }),
      });
      return new GamaEditorPane(options.textFileService, paneOptions);
    },
  });
}

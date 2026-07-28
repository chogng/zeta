import type {
  ConfigurationMainService,
} from "../../configuration/electron-main/configurationMainService.js";
import {
  validateKeybindingsResource,
} from "../common/keybindingsResource.js";
import type {
  KeybindingsResourceMainService,
} from "./keybindingsResourceMainService.js";

const legacyConfigurationKey = "keyboard.keybindings";

/**
 * Moves the former configuration value into the standalone keybinding file.
 *
 * An existing non-empty keybinding resource wins. The legacy value is only
 * removed after it validates and any required standalone write succeeds.
 */
export async function migrateLegacyKeybindings(
  configuration: ConfigurationMainService,
  keybindings: KeybindingsResourceMainService,
): Promise<boolean> {
  const configurationSnapshot = configuration.read();
  if (
    !Object.hasOwn(
      configurationSnapshot.document.values,
      legacyConfigurationKey,
    )
  ) {
    return false;
  }

  const migrated = validateKeybindingsResource(
    configurationSnapshot.document.values[legacyConfigurationKey],
  );
  const keybindingsSnapshot = keybindings.read();
  if (
    keybindingsSnapshot.bindings.length === 0 &&
    migrated.length > 0
  ) {
    await keybindings.update({
      expectedRevision: keybindingsSnapshot.revision,
      bindings: migrated,
    });
  }

  const values = {
    ...configurationSnapshot.document.values,
  };
  delete values[legacyConfigurationKey];
  await configuration.update({
    expectedRevision: configurationSnapshot.revision,
    document: {
      version: 1,
      values,
    },
  });
  return true;
}

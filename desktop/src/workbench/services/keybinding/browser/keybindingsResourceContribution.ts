import {
  combinedDisposable,
  DisposableOwner,
  DisposableSlot,
  type IDisposable,
} from "../../../../base/common/lifecycle.js";
import { parseKeybinding } from "../../../../base/common/keybindingParser.js";
import {
  environment,
  type HostOperatingSystem,
} from "../../../../base/common/platform.js";
import {
  parseContextKeyExpression,
} from "../../../../platform/contextkey/common/contextKeyExpressionParser.js";
import {
  type KeybindingRegistry,
  KeybindingsRegistry,
  KeybindingWeight,
} from "../../../../platform/keybinding/common/keybindingsRegistry.js";
import type {
  IKeybindingEntry,
  IKeybindingsResourceService,
} from "../../../../platform/keybinding/common/keybindingsResource.js";

export interface KeybindingsResourceContributionOptions {
  readonly service: IKeybindingsResourceService;
  readonly registry?: KeybindingRegistry;
  readonly operatingSystem?: HostOperatingSystem;
}

/**
 * Projects the active keybinding resource into one window registry.
 *
 * Previous registrations remain installed until every new rule validates.
 * A failed replacement disposes its partial registrations and preserves the
 * last complete rule set.
 */
export class KeybindingsResourceContribution extends DisposableOwner {
  readonly #service: IKeybindingsResourceService;
  readonly #registry: KeybindingRegistry;
  readonly #operatingSystem: HostOperatingSystem;
  readonly #registration = this.own(new DisposableSlot<IDisposable>());

  constructor(options: KeybindingsResourceContributionOptions) {
    super();
    this.#service = options.service;
    this.#registry = options.registry ?? KeybindingsRegistry;
    this.#operatingSystem = options.operatingSystem ?? environment.os;
    this.#reload(this.#service.getKeybindings());
    this.own(this.#service.onDidChangeKeybindings((bindings) => {
      this.#reload(bindings);
    }));
  }

  #reload(bindings: readonly IKeybindingEntry[]): void {
    const registrations: IDisposable[] = [];
    try {
      for (const binding of bindings) {
        const key = operatingSystemKey(binding, this.#operatingSystem);
        if (key === null) continue;
        const keybinding = parseKeybinding(key);
        if (!keybinding) {
          throw new Error(`Invalid keybinding resource entry: ${key}`);
        }
        const when = binding.when === undefined
          ? undefined
          : parseContextKeyExpression(binding.when);
        registrations.push(binding.command === null
          ? this.#registry.registerKeybindingBlocker({
            keybinding,
            when,
            weight: KeybindingWeight.User,
          })
          : this.#registry.registerKeybindingRule({
            command: binding.command,
            keybinding,
            when,
            args: binding.args === undefined ? undefined : [binding.args],
            weight: KeybindingWeight.User,
          }));
      }
    } catch (error) {
      for (const registration of registrations.reverse()) {
        registration.dispose();
      }
      throw error;
    }
    this.#registration.replace(combineRegistrations(registrations));
  }
}

function operatingSystemKey(
  binding: IKeybindingEntry,
  target: HostOperatingSystem,
): string | null {
  switch (target) {
    case "mac":
      return binding.mac === undefined ? binding.key : binding.mac;
    case "windows":
      return binding.win === undefined ? binding.key : binding.win;
    case "linux":
      return binding.linux === undefined ? binding.key : binding.linux;
    case "unknown":
      return binding.key;
  }
}

function combineRegistrations(
  registrations: readonly IDisposable[],
): IDisposable {
  return combinedDisposable(...registrations);
}

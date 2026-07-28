import {
  type Keybinding,
  type KeybindingEvent,
  matchesResolvedChord,
  type ResolvedKeybinding,
  resolveKeybinding,
} from "../../../base/common/keybindings.js";
import type { Event } from "../../../base/common/event.js";
import type {
  CommandId,
} from "../../commands/common/command-registry.js";
import type {
  Context,
} from "../../contextkey/common/contextkey.js";
import {
  type IRegisteredKeybindingRule,
  type KeybindingRegistry,
  KeybindingsRegistry,
  KeybindingWeight,
} from "./keybindingsRegistry.js";

export enum KeybindingResolveKind {
  NoMatch = "noMatch",
  MoreChordsNeeded = "moreChordsNeeded",
  Command = "command",
}

export type KeybindingResolveResult =
  | { readonly kind: KeybindingResolveKind.NoMatch }
  | {
    readonly kind: KeybindingResolveKind.MoreChordsNeeded;
    readonly keybinding: ResolvedKeybinding;
  }
  | {
    readonly kind: KeybindingResolveKind.Command;
    readonly command: CommandId;
    readonly args: readonly unknown[];
    readonly keybinding: ResolvedKeybinding;
  };

interface ResolvedRule {
  readonly rule: IRegisteredKeybindingRule;
  readonly keybinding: ResolvedKeybinding;
}

export interface KeybindingResolverOptions {
  readonly registry?: KeybindingRegistry;
  readonly resolveKeybinding?: (
    keybinding: Keybinding,
  ) => ResolvedKeybinding;
}

/**
 * Resolves command conflicts and multi-chord prefixes against one context.
 */
export class KeybindingResolver {
  readonly #registry: KeybindingRegistry;
  readonly #resolveKeybinding: (
    keybinding: Keybinding,
  ) => ResolvedKeybinding;

  constructor(options: KeybindingResolverOptions = {}) {
    this.#registry = options.registry ?? KeybindingsRegistry;
    this.#resolveKeybinding = options.resolveKeybinding ??
      ((keybinding) => resolveKeybinding(keybinding));
  }

  get onDidChangeKeybindings(): Event<void> {
    return this.#registry.onDidChangeKeybindings;
  }

  resolve(
    context: Context,
    events: readonly KeybindingEvent[],
  ): KeybindingResolveResult {
    if (events.length === 0) {
      return { kind: KeybindingResolveKind.NoMatch };
    }

    const candidates = this.#resolvedRules()
      .filter(({ rule, keybinding }) =>
        (!rule.when || rule.when.evaluate(context)) &&
        keybinding.chords.length >= events.length &&
        events.every((event, index) =>
          matchesResolvedChord(keybinding.chords[index], event)
        )
      )
      .sort(compareResolvedRules);
    const winner = candidates[0];
    if (!winner) return { kind: KeybindingResolveKind.NoMatch };

    if (winner.keybinding.chords.length > events.length) {
      return {
        kind: KeybindingResolveKind.MoreChordsNeeded,
        keybinding: winner.keybinding,
      };
    }
    return {
      kind: KeybindingResolveKind.Command,
      command: winner.rule.command,
      args: winner.rule.args ?? [],
      keybinding: winner.keybinding,
    };
  }

  lookupKeybinding(
    command: CommandId,
    context: Context,
  ): ResolvedKeybinding | undefined {
    return this.#resolvedRules()
      .filter(({ rule }) =>
        rule.command === command &&
        (!rule.when || rule.when.evaluate(context))
      )
      .sort(compareResolvedRules)[0]?.keybinding;
  }

  lookupKeybindings(
    command: CommandId,
    context: Context,
  ): readonly ResolvedKeybinding[] {
    return this.#resolvedRules()
      .filter(({ rule }) =>
        rule.command === command &&
        (!rule.when || rule.when.evaluate(context))
      )
      .sort(compareResolvedRules)
      .map(({ keybinding }) => keybinding);
  }

  #resolvedRules(): readonly ResolvedRule[] {
    return this.#registry.getKeybindings().map((rule) => ({
      rule,
      keybinding: this.#resolveKeybinding(rule.keybinding),
    }));
  }
}

function compareResolvedRules(
  first: ResolvedRule,
  second: ResolvedRule,
): number {
  const weight = (second.rule.weight ?? KeybindingWeight.Workbench) -
    (first.rule.weight ?? KeybindingWeight.Workbench);
  return weight !== 0 ? weight : second.rule.order - first.rule.order;
}

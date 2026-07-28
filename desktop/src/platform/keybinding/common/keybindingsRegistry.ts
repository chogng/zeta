import { Emitter, type Event } from "../../../base/common/event.js";
import type {
  Keybinding,
} from "../../../base/common/keybindings.js";
import {
  type IDisposable,
  toDisposable,
} from "../../../base/common/lifecycle.js";
import type {
  CommandId,
} from "../../commands/common/command-registry.js";
import type {
  ContextKeyExpression,
} from "../../contextkey/common/contextkey.js";

export enum KeybindingWeight {
  Builtin = 0,
  Workbench = 100,
  User = 200,
}

export enum KeybindingRuleKind {
  Command = "command",
  Blocker = "blocker",
}

/** One command shortcut contribution before host-specific resolution. */
export interface IKeybindingRule {
  readonly command: CommandId;
  readonly keybinding: Keybinding;
  readonly when?: ContextKeyExpression;
  readonly args?: readonly unknown[];
  readonly weight?: number;
}

/** Explicitly consumes a shortcut without dispatching a command. */
export interface IKeybindingBlocker {
  readonly keybinding: Keybinding;
  readonly when?: ContextKeyExpression;
  readonly weight?: number;
}

export interface IRegisteredCommandKeybindingRule
  extends IKeybindingRule {
  readonly kind: KeybindingRuleKind.Command;
  readonly order: number;
}

export interface IRegisteredKeybindingBlocker
  extends IKeybindingBlocker {
  readonly kind: KeybindingRuleKind.Blocker;
  readonly order: number;
}

export type IRegisteredKeybindingRule =
  | IRegisteredCommandKeybindingRule
  | IRegisteredKeybindingBlocker;

/** Stores realm-wide keybinding contributions and their override order. */
export class KeybindingRegistry {
  readonly #rules: IRegisteredKeybindingRule[] = [];
  readonly #onDidChangeKeybindings = new Emitter<void>();
  #nextOrder = 1;

  readonly onDidChangeKeybindings: Event<void> =
    this.#onDidChangeKeybindings.event;

  registerKeybindingRule(rule: IKeybindingRule): IDisposable {
    const registered: IRegisteredCommandKeybindingRule = {
      ...rule,
      kind: KeybindingRuleKind.Command,
      order: this.#nextOrder++,
    };
    return this.#register(registered);
  }

  registerKeybindingBlocker(
    blocker: IKeybindingBlocker,
  ): IDisposable {
    const registered: IRegisteredKeybindingBlocker = {
      ...blocker,
      kind: KeybindingRuleKind.Blocker,
      order: this.#nextOrder++,
    };
    return this.#register(registered);
  }

  #register(registered: IRegisteredKeybindingRule): IDisposable {
    this.#rules.push(registered);
    this.#onDidChangeKeybindings.fire();
    return toDisposable(() => {
      const index = this.#rules.indexOf(registered);
      if (index < 0) return;
      this.#rules.splice(index, 1);
      this.#onDidChangeKeybindings.fire();
    });
  }

  getKeybindings(): readonly IRegisteredKeybindingRule[] {
    return [...this.#rules];
  }
}

/** Realm-wide keybinding contributions populated by action modules. */
export const KeybindingsRegistry = new KeybindingRegistry();

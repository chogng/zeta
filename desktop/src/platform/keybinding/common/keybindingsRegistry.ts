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

/** One command shortcut contribution before host-specific resolution. */
export interface IKeybindingRule {
  readonly command: CommandId;
  readonly keybinding: Keybinding;
  readonly when?: ContextKeyExpression;
  readonly args?: readonly unknown[];
  readonly weight?: number;
}

export interface IRegisteredKeybindingRule extends IKeybindingRule {
  readonly order: number;
}

/** Stores realm-wide keybinding contributions and their override order. */
export class KeybindingRegistry {
  readonly #rules: IRegisteredKeybindingRule[] = [];
  readonly #onDidChangeKeybindings = new Emitter<void>();
  #nextOrder = 1;

  readonly onDidChangeKeybindings: Event<void> =
    this.#onDidChangeKeybindings.event;

  registerKeybindingRule(rule: IKeybindingRule): IDisposable {
    const registered: IRegisteredKeybindingRule = {
      ...rule,
      order: this.#nextOrder++,
    };
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

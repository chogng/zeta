import { Emitter } from "../../../base/common/event.js";
import { toDisposable, } from "../../../base/common/lifecycle.js";
export var KeybindingWeight;
(function (KeybindingWeight) {
    KeybindingWeight[KeybindingWeight["Builtin"] = 0] = "Builtin";
    KeybindingWeight[KeybindingWeight["Workbench"] = 100] = "Workbench";
    KeybindingWeight[KeybindingWeight["User"] = 200] = "User";
})(KeybindingWeight || (KeybindingWeight = {}));
export var KeybindingRuleKind;
(function (KeybindingRuleKind) {
    KeybindingRuleKind["Command"] = "command";
    KeybindingRuleKind["Blocker"] = "blocker";
})(KeybindingRuleKind || (KeybindingRuleKind = {}));
/** Stores realm-wide keybinding contributions and their override order. */
export class KeybindingRegistry {
    #rules = [];
    #onDidChangeKeybindings = new Emitter();
    #nextOrder = 1;
    onDidChangeKeybindings = this.#onDidChangeKeybindings.event;
    registerKeybindingRule(rule) {
        const registered = {
            ...rule,
            kind: KeybindingRuleKind.Command,
            order: this.#nextOrder++,
        };
        return this.#register(registered);
    }
    registerKeybindingBlocker(blocker) {
        const registered = {
            ...blocker,
            kind: KeybindingRuleKind.Blocker,
            order: this.#nextOrder++,
        };
        return this.#register(registered);
    }
    #register(registered) {
        this.#rules.push(registered);
        this.#onDidChangeKeybindings.fire();
        return toDisposable(() => {
            const index = this.#rules.indexOf(registered);
            if (index < 0)
                return;
            this.#rules.splice(index, 1);
            this.#onDidChangeKeybindings.fire();
        });
    }
    getKeybindings() {
        return [...this.#rules];
    }
}
/** Realm-wide keybinding contributions populated by action modules. */
export const KeybindingsRegistry = new KeybindingRegistry();

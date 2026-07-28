import { combinedDisposable, DisposableOwner, DisposableSlot, } from "../../../../base/common/lifecycle.js";
import { parseKeybinding } from "../../../../base/common/keybindingParser.js";
import { environment, } from "../../../../base/common/platform.js";
import { parseContextKeyExpression, } from "../../../../platform/contextkey/common/contextKeyExpressionParser.js";
import { KeybindingsRegistry, KeybindingWeight, } from "../../../../platform/keybinding/common/keybindingsRegistry.js";
/**
 * Projects the active keybinding resource into one window registry.
 *
 * Previous registrations remain installed until every new rule validates.
 * A failed replacement disposes its partial registrations and preserves the
 * last complete rule set.
 */
export class KeybindingsResourceContribution extends DisposableOwner {
    #service;
    #registry;
    #operatingSystem;
    #registration = this.own(new DisposableSlot());
    constructor(options) {
        super();
        this.#service = options.service;
        this.#registry = options.registry ?? KeybindingsRegistry;
        this.#operatingSystem = options.operatingSystem ?? environment.os;
        this.#reload(this.#service.getKeybindings());
        this.own(this.#service.onDidChangeKeybindings((bindings) => {
            this.#reload(bindings);
        }));
    }
    #reload(bindings) {
        const registrations = [];
        try {
            for (const binding of bindings) {
                const key = operatingSystemKey(binding, this.#operatingSystem);
                if (key === null)
                    continue;
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
        }
        catch (error) {
            for (const registration of registrations.reverse()) {
                registration.dispose();
            }
            throw error;
        }
        this.#registration.replace(combineRegistrations(registrations));
    }
}
function operatingSystemKey(binding, target) {
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
function combineRegistrations(registrations) {
    return combinedDisposable(...registrations);
}

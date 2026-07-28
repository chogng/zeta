import { matchesResolvedChord, resolveKeybinding, } from "../../../base/common/keybindings.js";
import { KeybindingsRegistry, KeybindingRuleKind, KeybindingWeight, } from "./keybindingsRegistry.js";
export var KeybindingResolveKind;
(function (KeybindingResolveKind) {
    KeybindingResolveKind["NoMatch"] = "noMatch";
    KeybindingResolveKind["MoreChordsNeeded"] = "moreChordsNeeded";
    KeybindingResolveKind["Command"] = "command";
    KeybindingResolveKind["Blocked"] = "blocked";
})(KeybindingResolveKind || (KeybindingResolveKind = {}));
/**
 * Resolves command conflicts and multi-chord prefixes against one context.
 */
export class KeybindingResolver {
    #registry;
    #resolveKeybinding;
    constructor(options = {}) {
        this.#registry = options.registry ?? KeybindingsRegistry;
        this.#resolveKeybinding = options.resolveKeybinding ??
            ((keybinding) => resolveKeybinding(keybinding));
    }
    get onDidChangeKeybindings() {
        return this.#registry.onDidChangeKeybindings;
    }
    resolve(context, events) {
        if (events.length === 0) {
            return { kind: KeybindingResolveKind.NoMatch };
        }
        const candidates = this.#resolvedRules()
            .filter(({ rule, keybinding }) => (!rule.when || rule.when.evaluate(context)) &&
            keybinding.chords.length >= events.length &&
            events.every((event, index) => matchesResolvedChord(keybinding.chords[index], event)))
            .sort(compareResolvedRules);
        const winner = candidates[0];
        if (!winner)
            return { kind: KeybindingResolveKind.NoMatch };
        if (winner.keybinding.chords.length > events.length) {
            return {
                kind: KeybindingResolveKind.MoreChordsNeeded,
                keybinding: winner.keybinding,
            };
        }
        if (winner.rule.kind === KeybindingRuleKind.Blocker) {
            return {
                kind: KeybindingResolveKind.Blocked,
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
    lookupKeybinding(command, context) {
        return this.#winningRules(context)
            .find(({ rule }) => rule.kind === KeybindingRuleKind.Command &&
            rule.command === command)?.keybinding;
    }
    lookupKeybindings(command, context) {
        return this.#winningRules(context)
            .filter(({ rule }) => rule.kind === KeybindingRuleKind.Command &&
            rule.command === command)
            .map(({ keybinding }) => keybinding);
    }
    #winningRules(context) {
        const winners = new Map();
        for (const candidate of this.#resolvedRules()
            .filter(({ rule }) => !rule.when || rule.when.evaluate(context))
            .sort(compareResolvedRules)) {
            const identity = keybindingIdentity(candidate.keybinding);
            if (!winners.has(identity))
                winners.set(identity, candidate);
        }
        return [...winners.values()];
    }
    #resolvedRules() {
        return this.#registry.getKeybindings().map((rule) => ({
            rule,
            keybinding: this.#resolveKeybinding(rule.keybinding),
        }));
    }
}
function keybindingIdentity(keybinding) {
    return keybinding.chords.map((chord) => [
        chord.kind,
        chord.key,
        Number(chord.ctrlKey),
        Number(chord.shiftKey),
        Number(chord.altKey),
        Number(chord.metaKey),
    ].join(":")).join(" ");
}
function compareResolvedRules(first, second) {
    const weight = (second.rule.weight ?? KeybindingWeight.Workbench) -
        (first.rule.weight ?? KeybindingWeight.Workbench);
    return weight !== 0 ? weight : second.rule.order - first.rule.order;
}

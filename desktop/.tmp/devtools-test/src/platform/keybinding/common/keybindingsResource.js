import { validateJsonValue, } from "../../../base/common/jsonValue.js";
import { parseKeybinding } from "../../../base/common/keybindingParser.js";
import { parseContextKeyExpression, } from "../../contextkey/common/contextKeyExpressionParser.js";
import { createServiceIdentifier, } from "../../instantiation/common/instantiation.js";
export const KEYBINDINGS_RESOURCE_READ_CHANNEL = "zeta:keybindings-resource:read";
export const KEYBINDINGS_RESOURCE_UPDATE_CHANNEL = "zeta:keybindings-resource:update";
export const KEYBINDINGS_RESOURCE_CHANGED_CHANNEL = "zeta:keybindings-resource:changed";
export const IKeybindingsResourceService = createServiceIdentifier("keybindingsResourceService");
/** Validates the complete ordered contents of `keybindings.json`. */
export function validateKeybindingsResource(value) {
    if (!Array.isArray(value)) {
        throw new TypeError("Keybindings resource must be an array");
    }
    if (value.length > 1_024) {
        throw new TypeError("Keybindings resource contains too many rules");
    }
    return value.map((candidate, index) => validateKeybindingEntry(candidate, index));
}
export function validateKeybindingsResourceSnapshot(value) {
    const snapshot = exactRecord(value, ["bindings", "revision"], "keybindings resource snapshot");
    return {
        revision: nonNegativeSafeInteger(snapshot.revision, "revision"),
        bindings: validateKeybindingsResource(snapshot.bindings),
    };
}
export function validateKeybindingsResourceUpdateRequest(value) {
    const request = exactRecord(value, ["bindings", "expectedRevision"], "keybindings resource update");
    return {
        expectedRevision: nonNegativeSafeInteger(request.expectedRevision, "expectedRevision"),
        bindings: validateKeybindingsResource(request.bindings),
    };
}
export function validateKeybindingsResourceRead(value) {
    if (value !== undefined) {
        throw new Error("Keybindings resource read does not accept parameters");
    }
    return undefined;
}
function validateKeybindingEntry(value, index) {
    const path = `keybindings[${index}]`;
    const source = record(value, path);
    const allowedKeys = new Set([
        "args",
        "command",
        "key",
        "linux",
        "mac",
        "when",
        "win",
    ]);
    for (const field of Object.keys(source)) {
        if (!allowedKeys.has(field)) {
            throw new TypeError(`${path} contains unknown field '${field}'`);
        }
    }
    if (!Object.hasOwn(source, "key") ||
        !Object.hasOwn(source, "command")) {
        throw new TypeError(`${path} requires key and command`);
    }
    const key = validateKey(source.key, `${path}.key`);
    const command = validateCommand(source.command, `${path}.command`);
    const when = optionalString(source.when, `${path}.when`, 1_024);
    if (when !== undefined)
        parseContextKeyExpression(when);
    const args = Object.hasOwn(source, "args")
        ? validateJsonValue(source.args, {
            path: `${path}.args`,
            maxDepth: 8,
            maxNodes: 2_048,
            maxStringLength: 16 * 1_024,
        })
        : undefined;
    if (command === null && args !== undefined) {
        throw new TypeError(`${path}.args requires a command`);
    }
    const mac = optionalKey(source.mac, `${path}.mac`);
    const linux = optionalKey(source.linux, `${path}.linux`);
    const win = optionalKey(source.win, `${path}.win`);
    return {
        key,
        command,
        ...(when === undefined ? {} : { when }),
        ...(args === undefined ? {} : { args }),
        ...(mac === undefined ? {} : { mac }),
        ...(linux === undefined ? {} : { linux }),
        ...(win === undefined ? {} : { win }),
    };
}
function validateCommand(value, path) {
    if (value === null)
        return null;
    if (typeof value !== "string" ||
        value.trim().length === 0 ||
        value.length > 256) {
        throw new TypeError(`${path} must be a non-empty command id or null`);
    }
    return value;
}
function optionalString(value, path, maxLength) {
    if (value === undefined)
        return undefined;
    if (typeof value !== "string" ||
        value.trim().length === 0 ||
        value.length > maxLength) {
        throw new TypeError(`${path} must be a non-empty bounded string`);
    }
    return value;
}
function optionalKey(value, path) {
    if (value === undefined || value === null)
        return value;
    return validateKey(value, path);
}
function validateKey(value, path) {
    if (typeof value !== "string" ||
        value.length > 256) {
        throw new TypeError(`${path} must be a valid keybinding`);
    }
    const keybinding = parseKeybinding(value);
    if (!keybinding || keybinding.chords.length > 4) {
        throw new TypeError(`${path} must be a valid keybinding`);
    }
    return value;
}
function exactRecord(value, keys, path) {
    const result = record(value, path);
    const actual = Object.keys(result).sort();
    const expected = [...keys].sort();
    if (actual.length !== expected.length ||
        actual.some((key, index) => key !== expected[index])) {
        throw new Error(`${path} must contain exactly: ${expected.join(", ")}`);
    }
    return result;
}
function record(value, path) {
    if (typeof value !== "object" || value === null || Array.isArray(value)) {
        throw new TypeError(`${path} must be an object`);
    }
    return value;
}
function nonNegativeSafeInteger(value, field) {
    if (!Number.isSafeInteger(value) || value < 0) {
        throw new Error(`${field} must be a non-negative safe integer`);
    }
    return value;
}

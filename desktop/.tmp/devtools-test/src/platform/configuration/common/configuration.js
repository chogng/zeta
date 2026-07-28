import { validateJsonValue, } from "../../../base/common/jsonValue.js";
import { createServiceIdentifier, } from "../../instantiation/common/instantiation.js";
export const IConfigurationService = createServiceIdentifier("configurationService");
export const CONFIGURATION_READ_CHANNEL = "zeta:configuration:read";
export const CONFIGURATION_UPDATE_CHANNEL = "zeta:configuration:update";
export const CONFIGURATION_CHANGED_CHANNEL = "zeta:configuration:changed";
export function emptyConfigurationDocument() {
    return {
        version: 1,
        values: {},
    };
}
/** Validates an untrusted persisted configuration document. */
export function validateConfigurationDocument(value) {
    const document = exactRecord(value, ["values", "version"]);
    if (document.version !== 1) {
        throw new Error("configuration version must be 1");
    }
    const values = record(document.values, "values");
    const validated = {};
    for (const [key, candidate] of Object.entries(values)) {
        if (!/^[A-Za-z][A-Za-z0-9.-]{0,127}$/.test(key)) {
            throw new Error(`invalid configuration key: ${key}`);
        }
        validated[key] = validateJsonValue(candidate, {
            path: `values.${key}`,
        });
    }
    return {
        version: 1,
        values: validated,
    };
}
export function validateConfigurationSnapshot(value) {
    const snapshot = exactRecord(value, ["document", "revision"]);
    return {
        revision: nonNegativeSafeInteger(snapshot.revision, "revision"),
        document: validateConfigurationDocument(snapshot.document),
    };
}
export function validateConfigurationUpdateRequest(value) {
    const request = exactRecord(value, ["document", "expectedRevision"]);
    return {
        expectedRevision: nonNegativeSafeInteger(request.expectedRevision, "expectedRevision"),
        document: validateConfigurationDocument(request.document),
    };
}
export function validateConfigurationRead(value) {
    if (value !== undefined) {
        throw new Error("configuration read does not accept parameters");
    }
    return undefined;
}
function exactRecord(value, keys) {
    const result = record(value, "configuration");
    const actual = Object.keys(result).sort();
    const expected = [...keys].sort();
    if (actual.length !== expected.length ||
        actual.some((key, index) => key !== expected[index])) {
        throw new Error(`configuration object must contain exactly: ${expected.join(", ")}`);
    }
    return result;
}
function record(value, path) {
    if (typeof value !== "object" || value === null || Array.isArray(value)) {
        throw new Error(`${path} must be an object`);
    }
    return value;
}
function nonNegativeSafeInteger(value, field) {
    if (!Number.isSafeInteger(value) || value < 0) {
        throw new Error(`${field} must be a non-negative safe integer`);
    }
    return value;
}

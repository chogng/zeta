const UUID_PATTERN = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
/** Returns whether an unknown value is a non-nil RFC 9562 UUID. */
export function isUuid(value) {
    return typeof value === "string" && UUID_PATTERN.test(value);
}
/** Validates an external UUID value and returns its canonical representation. */
export function parseUuid(value) {
    if (!isUuid(value)) {
        throw new TypeError("Expected a valid UUID");
    }
    return value.toLowerCase();
}
/** Creates a cryptographically random UUID. */
export function createUuid() {
    return globalThis.crypto.randomUUID();
}

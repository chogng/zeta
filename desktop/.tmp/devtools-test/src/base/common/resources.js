import { isMacintosh, isWindows } from "./platform.js";
/** Path comparison behavior selected for a resource. */
export var ResourcePathCasing;
(function (ResourcePathCasing) {
    ResourcePathCasing["Sensitive"] = "sensitive";
    ResourcePathCasing["Insensitive"] = "insensitive";
})(ResourcePathCasing || (ResourcePathCasing = {}));
const UNRESERVED_CHARACTER = /^[A-Za-z\d\-._~]$/;
function normalizePercentEncoding(value) {
    return value.replace(/%([0-9a-f]{2})/gi, (_match, hexadecimal) => {
        const character = String.fromCharCode(Number.parseInt(hexadecimal, 16));
        return UNRESERVED_CHARACTER.test(character)
            ? character
            : `%${hexadecimal.toUpperCase()}`;
    });
}
function keyPart(value) {
    return `${value.length}:${value}`;
}
/**
 * Implements URI comparison with caller-selected path casing semantics.
 */
export class ExtUri {
    #pathCasing;
    constructor(pathCasing) {
        this.#pathCasing = pathCasing;
    }
    getComparisonKey(uri) {
        return this.#createComparisonKey(uri, true);
    }
    getComparisonKeyIgnoringFragment(uri) {
        return this.#createComparisonKey(uri, false);
    }
    isEqual(left, right) {
        if (left === right)
            return true;
        if (!left || !right)
            return false;
        return this.getComparisonKey(left) === this.getComparisonKey(right);
    }
    isEqualIgnoringFragment(left, right) {
        if (left === right)
            return true;
        if (!left || !right)
            return false;
        return this.getComparisonKeyIgnoringFragment(left)
            === this.getComparisonKeyIgnoringFragment(right);
    }
    ignorePathCasing(uri) {
        return this.#pathCasing(uri) === ResourcePathCasing.Insensitive;
    }
    #createComparisonKey(uri, includeFragment) {
        const path = this.ignorePathCasing(uri)
            ? decodeURIComponent(uri.path).toLowerCase()
            : normalizePercentEncoding(uri.path);
        const fragment = includeFragment
            ? normalizePercentEncoding(uri.fragment)
            : "";
        return [
            keyPart(uri.scheme.toLowerCase()),
            keyPart(uri.authority.toLowerCase()),
            keyPart(path),
            keyPart(normalizePercentEncoding(uri.query)),
            keyPart(fragment),
        ].join("");
    }
}
/** URI identity that preserves path casing. */
export const extUri = new ExtUri(() => ResourcePathCasing.Sensitive);
/**
 * URI identity biased toward the current native file-system path casing.
 *
 * Remote providers should supply an `ExtUri` matching their own semantics.
 */
export const extUriBiasedIgnorePathCase = new ExtUri((uri) => {
    if (uri.scheme === "file"
        && (isWindows || isMacintosh)) {
        return ResourcePathCasing.Insensitive;
    }
    return ResourcePathCasing.Sensitive;
});
/** URI identity that ignores path casing for every scheme. */
export const extUriIgnorePathCase = new ExtUri(() => ResourcePathCasing.Insensitive);

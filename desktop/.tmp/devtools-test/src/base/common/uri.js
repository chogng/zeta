const URI_SCHEME = /^[A-Za-z][A-Za-z\d+.-]*:/;
const WINDOWS_DRIVE_PATH = /^[A-Za-z]:[\\/]/;
const WINDOWS_URI_DRIVE_PATH = /^\/[A-Za-z]:\//;
function parseUrl(value) {
    if (WINDOWS_DRIVE_PATH.test(value)) {
        throw new TypeError(`Windows paths must be created with URI.file(): ${value}`);
    }
    if (!URI_SCHEME.test(value)) {
        throw new TypeError(`URI must be absolute: ${value}`);
    }
    let url;
    try {
        url = new URL(value);
    }
    catch (error) {
        throw new TypeError(`Invalid URI: ${value}`, { cause: error });
    }
    if (url.username || url.password) {
        throw new TypeError("Resource URIs must not contain credentials");
    }
    validatePercentEncoding(url.pathname, "path");
    validatePercentEncoding(url.search, "query");
    validatePercentEncoding(url.hash, "fragment");
    return url;
}
function validatePercentEncoding(value, component) {
    try {
        decodeURIComponent(value);
    }
    catch (error) {
        throw new TypeError(`URI ${component} has invalid percent encoding`, {
            cause: error,
        });
    }
}
/**
 * An immutable absolute resource URI.
 *
 * Component accessors retain URI percent encoding so values round-trip without
 * changing reserved characters. Use `fsPath` when a decoded native path is
 * required for a `file:` URI.
 */
export class URI {
    #url;
    constructor(url) {
        this.#url = url;
    }
    /** Parses and canonicalizes an absolute URI. */
    static parse(value) {
        return new URI(parseUrl(value));
    }
    /**
     * Creates a `file:` URI from an absolute POSIX, Windows drive, or UNC path.
     */
    static file(path) {
        if (path.length === 0) {
            throw new TypeError("File path must not be empty");
        }
        const normalized = path.replaceAll("\\", "/");
        if (normalized.startsWith("//")) {
            const withoutPrefix = normalized.replace(/^\/+/, "");
            const separator = withoutPrefix.indexOf("/");
            const authority = separator < 0
                ? withoutPrefix
                : withoutPrefix.slice(0, separator);
            const resourcePath = separator < 0 ? "/" : withoutPrefix.slice(separator);
            if (!authority) {
                throw new TypeError(`UNC path must contain a host: ${path}`);
            }
            const url = new URL(`file://${authority}/`);
            url.pathname = resourcePath;
            return new URI(url);
        }
        if (!normalized.startsWith("/") && !WINDOWS_DRIVE_PATH.test(path)) {
            throw new TypeError(`File path must be absolute: ${path}`);
        }
        const resourcePath = WINDOWS_DRIVE_PATH.test(path)
            ? `/${normalized}`
            : normalized;
        const url = new URL("file:///");
        url.pathname = resourcePath;
        return new URI(url);
    }
    get scheme() {
        return this.#url.protocol.slice(0, -1).toLowerCase();
    }
    get authority() {
        return this.#url.host;
    }
    get path() {
        return this.#url.pathname;
    }
    get query() {
        return this.#url.search.slice(1);
    }
    get fragment() {
        return this.#url.hash.slice(1);
    }
    /**
     * Returns the decoded native path represented by a `file:` URI.
     */
    get fsPath() {
        if (this.scheme !== "file") {
            throw new TypeError(`URI scheme is not file: ${this.scheme}`);
        }
        const decodedPath = decodeURIComponent(this.#url.pathname);
        if (this.#url.host) {
            return `\\\\${this.#url.host}${decodedPath.replaceAll("/", "\\")}`;
        }
        if (WINDOWS_URI_DRIVE_PATH.test(decodedPath)) {
            return decodedPath.slice(1).replaceAll("/", "\\");
        }
        return decodedPath;
    }
    /** Returns a copy with a different percent-encoded path. */
    withPath(path) {
        const url = new URL(this.#url.href);
        url.pathname = path;
        return new URI(parseUrl(url.href));
    }
    /** Returns a copy with a different percent-encoded query. */
    withQuery(query) {
        const url = new URL(this.#url.href);
        url.search = query.length === 0 ? "" : `?${query}`;
        return new URI(parseUrl(url.href));
    }
    /** Returns a copy without a query component. */
    withoutQuery() {
        return this.withQuery("");
    }
    /** Returns a copy with a different percent-encoded fragment. */
    withFragment(fragment) {
        const url = new URL(this.#url.href);
        url.hash = fragment.length === 0 ? "" : `#${fragment}`;
        return new URI(parseUrl(url.href));
    }
    /** Returns a copy without a fragment component. */
    withoutFragment() {
        return this.withFragment("");
    }
    toString() {
        return this.#url.href;
    }
    toJSON() {
        return this.toString();
    }
}

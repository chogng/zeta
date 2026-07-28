export const BROWSER_VIEW_CREATE_CHANNEL = "zeta:browser-view:create";
export const BROWSER_VIEW_STATE_CHANNEL = "zeta:browser-view:state";
export const BROWSER_VIEW_LAYOUT_CHANNEL = "zeta:browser-view:layout";
export const BROWSER_VIEW_VISIBILITY_CHANNEL = "zeta:browser-view:visibility";
export const BROWSER_VIEW_NAVIGATE_CHANNEL = "zeta:browser-view:navigate";
export const BROWSER_VIEW_GO_BACK_CHANNEL = "zeta:browser-view:go-back";
export const BROWSER_VIEW_GO_FORWARD_CHANNEL = "zeta:browser-view:go-forward";
export const BROWSER_VIEW_RELOAD_CHANNEL = "zeta:browser-view:reload";
export const BROWSER_VIEW_STOP_CHANNEL = "zeta:browser-view:stop";
export const BROWSER_VIEW_CLOSE_CHANNEL = "zeta:browser-view:close";
export const BROWSER_VIEW_EVENT_CHANNEL = "zeta:browser-view:event";
const MAX_URL_LENGTH = 8192;
const MAX_BOUND_MAGNITUDE = 100_000;
const TARGET_ID_PATTERN = /^browser_target_[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;
export function validateBrowserViewCreateRequest(value) {
    const request = exactRecord(value, ["url"], "browser view create request");
    return { url: normalizeBrowserViewUrl(request.url) };
}
export function validateBrowserViewTargetRequest(value) {
    const request = exactRecord(value, ["targetId"], "browser view target request");
    return { targetId: validateTargetId(request.targetId) };
}
export function validateBrowserViewLayoutRequest(value) {
    const request = exactRecord(value, ["bounds", "targetId"], "browser view layout request");
    const bounds = exactRecord(request.bounds, ["height", "width", "x", "y"], "browser view bounds");
    return {
        targetId: validateTargetId(request.targetId),
        bounds: {
            x: boundedInteger(bounds.x, "bounds.x", true),
            y: boundedInteger(bounds.y, "bounds.y", true),
            width: boundedInteger(bounds.width, "bounds.width", false),
            height: boundedInteger(bounds.height, "bounds.height", false),
        },
    };
}
export function validateBrowserViewVisibilityRequest(value) {
    const request = exactRecord(value, ["targetId", "visible"], "browser view visibility request");
    if (typeof request.visible !== "boolean") {
        throw new Error("browser view visible must be a boolean");
    }
    return {
        targetId: validateTargetId(request.targetId),
        visible: request.visible,
    };
}
export function validateBrowserViewNavigateRequest(value) {
    const request = exactRecord(value, ["targetId", "url"], "browser view navigate request");
    return {
        targetId: validateTargetId(request.targetId),
        url: normalizeBrowserViewUrl(request.url),
    };
}
/** Normalizes URLs accepted by the embedded browser origin policy. */
export function normalizeBrowserViewUrl(value) {
    if (typeof value !== "string" || value.length === 0) {
        throw new Error("browser view URL must be a non-empty string");
    }
    if (value.length > MAX_URL_LENGTH) {
        throw new Error("browser view URL is too long");
    }
    let url;
    try {
        url = new URL(value);
    }
    catch {
        throw new Error("browser view URL is invalid");
    }
    if (url.username || url.password) {
        throw new Error("browser view URL credentials are not allowed");
    }
    const localHttpHost = url.hostname === "localhost" ||
        url.hostname === "127.0.0.1" ||
        url.hostname === "[::1]";
    if (url.protocol !== "https:" &&
        !(url.protocol === "http:" && localHttpHost) &&
        url.href !== "about:blank") {
        throw new Error("browser view URL must use HTTPS, loopback HTTP, or about:blank");
    }
    return url.href;
}
function validateTargetId(value) {
    if (typeof value !== "string" || !TARGET_ID_PATTERN.test(value)) {
        throw new Error("browser view targetId is invalid");
    }
    return value;
}
function boundedInteger(value, field, allowNegative) {
    if (!Number.isSafeInteger(value) ||
        Math.abs(value) > MAX_BOUND_MAGNITUDE ||
        (!allowNegative && value <= 0)) {
        throw new Error(`${field} is outside the supported integer range`);
    }
    return value;
}
function exactRecord(value, keys, label) {
    if (typeof value !== "object" || value === null || Array.isArray(value)) {
        throw new Error(`${label} must be an object`);
    }
    const result = value;
    const actual = Object.keys(result).sort();
    const expected = [...keys].sort();
    if (actual.length !== expected.length ||
        actual.some((key, index) => key !== expected[index])) {
        throw new Error(`${label} must contain exactly: ${expected.join(", ")}`);
    }
    return result;
}

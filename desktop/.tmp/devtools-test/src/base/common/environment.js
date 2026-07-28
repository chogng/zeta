/** @internal */
export function operatingSystemFromNodePlatform(platform) {
    switch (platform) {
        case "win32":
            return "windows";
        case "darwin":
            return "mac";
        case "linux":
            return "linux";
        default:
            return "unknown";
    }
}
/** @internal */
export function operatingSystemFromUserAgent(userAgent) {
    if (userAgent.includes("Windows"))
        return "windows";
    if (userAgent.includes("Macintosh")
        || userAgent.includes("iPhone")
        || userAgent.includes("iPad")) {
        return "mac";
    }
    if (userAgent.includes("Linux"))
        return "linux";
    return "unknown";
}

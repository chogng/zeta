export var Icon;
(function (Icon) {
    /** Creates an icon reference for an ID supplied by configuration or data. */
    function fromId(id) {
        return { id };
    }
    Icon.fromId = fromId;
})(Icon || (Icon = {}));
const iconDefaultsById = new Map();
const iconIdPattern = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;
/**
 * Registers the default artwork or semantic fallback for an icon ID.
 *
 * Product code should register an icon for the meaning it presents and use the
 * returned reference. Icon-library factories stay behind this boundary.
 */
export function registerIcon(id, defaults) {
    if (!iconIdPattern.test(id)) {
        throw new TypeError(`Invalid icon ID '${id}'`);
    }
    if (iconDefaultsById.has(id)) {
        throw new TypeError(`Icon '${id}' is already registered`);
    }
    iconDefaultsById.set(id, defaults);
    return Icon.fromId(id);
}
/** Resolves an icon reference to the SVG factory used by the browser layer. */
export function resolveIconDefinition(icon) {
    const visited = new Set();
    let current = icon;
    while (true) {
        if (visited.has(current.id)) {
            throw new Error(`Circular icon defaults: ${[...visited, current.id].join(" -> ")}`);
        }
        visited.add(current.id);
        const defaults = iconDefaultsById.get(current.id);
        if (!defaults) {
            throw new ReferenceError(`Unknown icon '${current.id}'`);
        }
        if (typeof defaults === "function")
            return defaults;
        current = defaults;
    }
}

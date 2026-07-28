export const productIds = [
    "code",
    "academic",
    "complete",
];
export const rendererEntryNames = [
    "workbench-code",
    "workbench-academic",
    "workbench-complete",
];
export const CodeProduct = {
    id: "code",
    name: "Zeta Code",
    rendererEntry: "workbench-code",
};
export const AcademicProduct = {
    id: "academic",
    name: "Zeta Academic",
    rendererEntry: "workbench-academic",
};
export const CompleteProduct = {
    id: "complete",
    name: "Zeta Complete",
    rendererEntry: "workbench-complete",
};
const products = {
    code: CodeProduct,
    academic: AcademicProduct,
    complete: CompleteProduct,
};
/** Resolves the selected product, defaulting local and legacy builds to Code. */
export function resolveProductId(value) {
    if (value === undefined || value.length === 0)
        return "code";
    if (isProductId(value))
        return value;
    throw new TypeError(`Unknown Zeta product '${value}'. Expected ${productIds.join(", ")}`);
}
export function getProductConfiguration(id) {
    return products[id];
}
function isProductId(value) {
    return productIds.includes(value);
}

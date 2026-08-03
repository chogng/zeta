const LANGUAGE_ID_PATTERN = /^[A-Za-z0-9][A-Za-z0-9._-]*$/;

/** Validates one concrete editor language identity. */
export function assertLanguageId(value: unknown): asserts value is string {
  if (typeof value !== "string" || !LANGUAGE_ID_PATTERN.test(value)) {
    throw new TypeError("Language ID must contain only letters, digits, dot, underscore, or hyphen");
  }
}

/** Validates a provider selector, including the explicit all-languages selector. */
export function assertLanguageSelector(value: unknown): asserts value is string {
  if (value === "*") return;
  assertLanguageId(value);
}

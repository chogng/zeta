export const USUAL_WORD_SEPARATORS = '`~!@#$%^&*()-=+[{]}\\|;:\'",.<>/?';

/** A word result expressed with zero-based UTF-16 columns. */
export interface IWordAtPosition {
  readonly word: string;
  readonly startColumnIndex: number;
  readonly endColumnIndexExclusive: number;
}

export interface IGetWordAtTextConfig {
  readonly maxLen: number;
  readonly windowSize: number;
  readonly timeBudget: number;
}

export const DEFAULT_WORD_REGEXP = createWordRegExp();

let defaultConfig: IGetWordAtTextConfig = { maxLen: 1000, windowSize: 15, timeBudget: 150 };

export function setDefaultGetWordAtTextConfig(value: IGetWordAtTextConfig): () => void {
  const previous = defaultConfig;
  defaultConfig = validateConfig(value);
  return () => { defaultConfig = previous; };
}

export function ensureValidWordDefinition(wordDefinition?: RegExp | null): RegExp {
  if (!wordDefinition) return DEFAULT_WORD_REGEXP;
  const flags = wordDefinition.flags.includes("g") ? wordDefinition.flags : `${wordDefinition.flags}g`;
  const result = wordDefinition.flags.includes("g") ? wordDefinition : new RegExp(wordDefinition.source, flags);
  result.lastIndex = 0;
  return result;
}

/** Finds the word-like regex match enclosing a zero-based text column. */
export function getWordAtText(columnIndex: number, wordDefinition: RegExp, text: string, textOffset = 0, config = defaultConfig): IWordAtPosition | null {
  validateConfig(config);
  if (!Number.isSafeInteger(columnIndex) || columnIndex < 0) throw new RangeError("columnIndex must be a non-negative safe integer");
  const regex = ensureValidWordDefinition(wordDefinition);
  if (text.length > config.maxLen) {
    const halfWindow = Math.floor(config.maxLen / 2);
    const start = Math.max(0, columnIndex - halfWindow);
    const end = Math.min(text.length, columnIndex + halfWindow);
    return getWordAtText(columnIndex - start, regex, text.slice(start, end), textOffset + start, config);
  }
  const probe = columnIndex - textOffset;
  const startedAt = Date.now();
  let match: RegExpExecArray | null;
  while ((match = regex.exec(text))) {
    if (Date.now() - startedAt >= config.timeBudget) break;
    const start = match.index;
    const end = start + match[0].length;
    if (match[0].length > 0 && start <= probe && probe < end) {
      regex.lastIndex = 0;
      return { word: match[0], startColumnIndex: textOffset + start, endColumnIndexExclusive: textOffset + end };
    }
    if (match[0].length === 0) regex.lastIndex += 1;
  }
  regex.lastIndex = 0;
  return null;
}

function createWordRegExp(allowInWords = ""): RegExp {
  let source = "(-?\\d*\\.\\d\\w*)|([^";
  for (const separator of USUAL_WORD_SEPARATORS) {
    if (!allowInWords.includes(separator)) source += `\\${separator}`;
  }
  return new RegExp(`${source}\\s]+)`, "g");
}

function validateConfig(config: IGetWordAtTextConfig): IGetWordAtTextConfig {
  if (!Number.isSafeInteger(config.maxLen) || config.maxLen < 1 || !Number.isSafeInteger(config.windowSize) || config.windowSize < 1 || !Number.isSafeInteger(config.timeBudget) || config.timeBudget < 1) {
    throw new RangeError("Invalid word lookup configuration");
  }
  return config;
}

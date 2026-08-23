/** Parses JSON with line comments, block comments, and trailing commas. */
export function parseJsonc(source: string, owner: string): unknown {
  if (typeof source !== "string") throw new TypeError(`${owner} must be text`);
  const withoutComments = stripComments(source, owner);
  const normalized = stripTrailingCommas(withoutComments);
  try {
    return JSON.parse(normalized);
  } catch (error) {
    throw new TypeError(`${owner} is not valid JSONC: ${error instanceof Error ? error.message : String(error)}`);
  }
}

function stripComments(source: string, owner: string): string {
  let result = "";
  let state: "normal" | "string" | "lineComment" | "blockComment" = "normal";
  let escaped = false;
  for (let index = 0; index < source.length; index += 1) {
    const character = source[index]!;
    const next = source[index + 1];
    if (state === "string") {
      result += character;
      if (escaped) escaped = false;
      else if (character === "\\") escaped = true;
      else if (character === '"') state = "normal";
      continue;
    }
    if (state === "lineComment") {
      if (character === "\n" || character === "\r") {
        result += character;
        state = "normal";
      }
      continue;
    }
    if (state === "blockComment") {
      if (character === "*" && next === "/") {
        index += 1;
        state = "normal";
      } else if (character === "\n" || character === "\r") {
        result += character;
      }
      continue;
    }
    if (character === '"') {
      result += character;
      state = "string";
    } else if (character === "/" && next === "/") {
      index += 1;
      state = "lineComment";
    } else if (character === "/" && next === "*") {
      index += 1;
      state = "blockComment";
    } else {
      result += character;
    }
  }
  if (state === "blockComment") throw new TypeError(`${owner} contains an unterminated block comment`);
  return result;
}

function stripTrailingCommas(source: string): string {
  let result = "";
  let state: "normal" | "string" = "normal";
  let escaped = false;
  for (let index = 0; index < source.length; index += 1) {
    const character = source[index]!;
    if (state === "string") {
      result += character;
      if (escaped) escaped = false;
      else if (character === "\\") escaped = true;
      else if (character === '"') state = "normal";
      continue;
    }
    if (character === '"') {
      result += character;
      state = "string";
      continue;
    }
    if (character === ",") {
      let lookahead = index + 1;
      while (/\s/u.test(source[lookahead] ?? "")) lookahead += 1;
      if (source[lookahead] === "}" || source[lookahead] === "]") continue;
    }
    result += character;
  }
  return result;
}

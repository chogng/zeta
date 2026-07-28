import { ContextKeyExpr, } from "./contextkey.js";
/**
 * Parses the user-facing subset of VS Code-style `when` expressions.
 *
 * Supported operators are `!`, `&&`, `||`, `==`, `===`, `!=`, `!==`, and
 * parentheses. Bare identifiers test truthiness; comparison values may be
 * quoted strings, bare words, booleans, null, or finite numbers.
 */
export function parseContextKeyExpression(source) {
    const parser = new Parser(source);
    return parser.parse();
}
class Parser {
    #source;
    #tokens;
    #position = 0;
    constructor(source) {
        this.#source = source;
        this.#tokens = tokenize(source);
    }
    parse() {
        if (this.#source.trim().length === 0) {
            throw new SyntaxError("Context key expression must not be empty");
        }
        const expression = this.#parseOr();
        this.#expect("end");
        return expression;
    }
    #parseOr() {
        const expressions = [this.#parseAnd()];
        while (this.#match("or"))
            expressions.push(this.#parseAnd());
        return ContextKeyExpr.or(...expressions);
    }
    #parseAnd() {
        const expressions = [this.#parseUnary()];
        while (this.#match("and"))
            expressions.push(this.#parseUnary());
        return ContextKeyExpr.and(...expressions);
    }
    #parseUnary() {
        if (this.#match("not")) {
            const operand = this.#parseUnary();
            return {
                evaluate: (context) => !operand.evaluate(context),
                keys: () => operand.keys(),
            };
        }
        if (this.#match("open")) {
            const expression = this.#parseOr();
            this.#expect("close");
            return expression;
        }
        return this.#parseComparison();
    }
    #parseComparison() {
        const identifier = this.#expect("identifier");
        const key = identifier.value;
        if (this.#match("equals")) {
            return ContextKeyExpr.equals(key, this.#comparisonValue());
        }
        if (this.#match("notEquals")) {
            return ContextKeyExpr.notEquals(key, this.#comparisonValue());
        }
        return ContextKeyExpr.has(key);
    }
    #comparisonValue() {
        const token = this.#current();
        if (token.kind !== "identifier" && token.kind !== "value") {
            throw this.#error(token, "Expected a comparison value");
        }
        this.#position += 1;
        return token.value;
    }
    #match(kind) {
        if (this.#current().kind !== kind)
            return false;
        this.#position += 1;
        return true;
    }
    #expect(kind) {
        const token = this.#current();
        if (token.kind !== kind) {
            throw this.#error(token, `Expected ${describeToken(kind)}`);
        }
        this.#position += 1;
        return token;
    }
    #current() {
        return this.#tokens[this.#position];
    }
    #error(token, message) {
        return new SyntaxError(`${message} at offset ${token.offset} in '${this.#source}'`);
    }
}
function tokenize(source) {
    const tokens = [];
    let offset = 0;
    while (offset < source.length) {
        const character = source[offset];
        if (/\s/.test(character)) {
            offset += 1;
            continue;
        }
        const operator = readOperator(source, offset);
        if (operator) {
            tokens.push(operator.token);
            offset = operator.nextOffset;
            continue;
        }
        if (character === "'" || character === "\"") {
            const quoted = readQuotedString(source, offset, character);
            tokens.push(quoted.token);
            offset = quoted.nextOffset;
            continue;
        }
        const start = offset;
        while (offset < source.length &&
            !/\s/.test(source[offset]) &&
            !"()!&|=".includes(source[offset])) {
            offset += 1;
        }
        if (start === offset) {
            throw new SyntaxError(`Unexpected '${source[offset]}' at offset ${offset}`);
        }
        const raw = source.slice(start, offset);
        tokens.push(classifyWord(raw, start));
    }
    tokens.push({ kind: "end", offset: source.length });
    return tokens;
}
function readOperator(source, offset) {
    const operators = [
        ["!==", "notEquals"],
        ["===", "equals"],
        ["!=", "notEquals"],
        ["==", "equals"],
        ["&&", "and"],
        ["||", "or"],
        ["!", "not"],
        ["(", "open"],
        [")", "close"],
    ];
    for (const [text, kind] of operators) {
        if (source.startsWith(text, offset)) {
            return {
                token: { kind, offset },
                nextOffset: offset + text.length,
            };
        }
    }
    return undefined;
}
function readQuotedString(source, offset, quote) {
    let current = offset + 1;
    let value = "";
    while (current < source.length) {
        const character = source[current];
        if (character === quote) {
            return {
                token: { kind: "value", offset, value },
                nextOffset: current + 1,
            };
        }
        if (character === "\\") {
            current += 1;
            if (current >= source.length)
                break;
            const escaped = source[current];
            value += escaped === "n"
                ? "\n"
                : escaped === "r"
                    ? "\r"
                    : escaped === "t"
                        ? "\t"
                        : escaped;
            current += 1;
            continue;
        }
        value += character;
        current += 1;
    }
    throw new SyntaxError(`Unterminated string at offset ${offset}`);
}
function classifyWord(raw, offset) {
    if (raw === "true")
        return { kind: "value", offset, value: true };
    if (raw === "false")
        return { kind: "value", offset, value: false };
    if (raw === "null")
        return { kind: "value", offset, value: null };
    if (/^-?(?:0|[1-9]\d*)(?:\.\d+)?$/.test(raw)) {
        const value = Number(raw);
        if (Number.isFinite(value)) {
            return { kind: "value", offset, value };
        }
    }
    if (!/^[A-Za-z_][A-Za-z0-9_.-]*$/.test(raw)) {
        throw new SyntaxError(`Invalid token '${raw}' at offset ${offset}`);
    }
    return { kind: "identifier", offset, value: raw };
}
function describeToken(kind) {
    switch (kind) {
        case "close":
            return "')'";
        case "end":
            return "the end of the expression";
        case "identifier":
            return "a context key";
        default:
            return kind;
    }
}

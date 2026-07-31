import { DisposableStore, type IDisposable } from "../../../base/common/lifecycle.js";
import { DEFAULT_LANGUAGE_AUTO_CLOSE_BEFORE, LanguageConfigurationRegistry, LanguageIndentAction, type LanguageAutoClosingPair, type LanguageAutoClosingTokenContext, type LanguageCharacterPair, type LanguageConfiguration, type LanguageConfigurationSource, type LanguageIndentationRules, type LanguageOnEnterRule, type ResolvedLanguageConfiguration } from "./languageConfiguration.js";
import { assertLanguageId } from "./languageId.js";

export const ALPHA_BUILTIN_LANGUAGE_IDS = Object.freeze([
  "typescript",
  "typescriptreact",
  "javascript",
  "javascriptreact",
  "json",
  "jsonc",
]);

const ECMASCRIPT_LANGUAGE_IDS = new Set(["typescript", "typescriptreact", "javascript", "javascriptreact"]);
const BRACKETS = Object.freeze([
  pair("(", ")"),
  pair("[", "]"),
  pair("{", "}"),
]);
const JSON_BRACKETS = Object.freeze([BRACKETS[1]!, BRACKETS[2]!]);
const ECMASCRIPT_PAIRS = Object.freeze([
  ...BRACKETS,
  autoPair("'", "'", ["string", "comment"]),
  autoPair("\"", "\"", ["string"]),
  autoPair("`", "`", ["string", "comment"]),
]);
const JSON_PAIRS = Object.freeze([
  ...JSON_BRACKETS,
  autoPair("\"", "\"", ["string"]),
]);
const ECMASCRIPT_SURROUNDING_PAIRS = Object.freeze(ECMASCRIPT_PAIRS.map(value => pair(value.open, value.close)));
const JSON_SURROUNDING_PAIRS = Object.freeze(JSON_PAIRS.map(value => pair(value.open, value.close)));
const ECMASCRIPT_INDENTATION_RULES: LanguageIndentationRules = Object.freeze({
  decreaseIndentPattern: /^\s*[\}\]\)].*$/,
  increaseIndentPattern: /^.*(\{[^}]*|\([^)]*|\[[^\]]*)$/,
  indentNextLinePattern: /^((.*=>\s*)|((.*[^\w]+|\s*)((if|while|for)\s*\(.*\)\s*|else\s*)))$/,
  unIndentedLinePattern: /^(\t|[ ])*[ ]\*[^/]*\*\/\s*$|^(\t|[ ])*[ ]\*\/\s*$|^(\t|[ ])*\*([ ]([^\*]|\*(?!\/))*)?$/,
});
const JSON_INDENTATION_RULES: LanguageIndentationRules = Object.freeze({
  increaseIndentPattern: /({+(?=((\\.|[^"\\])*"(\\.|[^"\\])*")*[^"}]*)$)|(\[+(?=((\\.|[^"\\])*"(\\.|[^"\\])*")*[^"\]]*)$)/,
  decreaseIndentPattern: /^\s*[}\]],?\s*$/,
});
const ECMASCRIPT_ON_ENTER_RULES: readonly LanguageOnEnterRule[] = Object.freeze([
  onEnter(/^\s*\/\*\*(?!\/)([^\*]|\*(?!\/))*$/, LanguageIndentAction.IndentOutdent, { afterText: /^\s*\*\/$/, appendText: " * " }),
  onEnter(/^\s*\/\*\*(?!\/)([^\*]|\*(?!\/))*$/, LanguageIndentAction.None, { appendText: " * " }),
  onEnter(/^(\t|[ ])*\*([ ]([^\*]|\*(?!\/))*)?$/, LanguageIndentAction.None, { previousLineText: /(?=^(\s*(\/\*\*|\*)).*)(?=(?!(\s*\*\/)))/, appendText: "* " }),
  onEnter(/^(\t|[ ])*[ ]\*\/\s*$/, LanguageIndentAction.None, { removeText: 1 }),
  onEnter(/^(\t|[ ])*[ ]\*[^/]*\*\/\s*$/, LanguageIndentAction.None, { removeText: 1 }),
  onEnter(/^\s*(\bcase\s.+:|\bdefault:)$/, LanguageIndentAction.Indent, { afterText: /^(?!\s*(\bcase\b|\bdefault\b))/ }),
]);
const JSONC_ON_ENTER_RULES: readonly LanguageOnEnterRule[] = Object.freeze([
  onEnter(/^\s*\/\/\s*\S|\s\/\/\s+\S/, LanguageIndentAction.None, { afterText: /^(?!\s*$)/, appendText: "// " }),
]);

interface BuiltinOnEnterOptions {
  readonly afterText?: RegExp;
  readonly previousLineText?: RegExp;
  readonly appendText?: string;
  readonly removeText?: number;
}

const ECMASCRIPT_CONFIGURATION: LanguageConfiguration = Object.freeze({
  comments: Object.freeze({
    lineComment: "//",
    blockComment: pair("/*", "*/"),
  }),
  brackets: BRACKETS,
  autoClosingPairs: ECMASCRIPT_PAIRS,
  surroundingPairs: ECMASCRIPT_SURROUNDING_PAIRS,
  indentationRules: ECMASCRIPT_INDENTATION_RULES,
  onEnterRules: ECMASCRIPT_ON_ENTER_RULES,
});
const JSON_CONFIGURATION: LanguageConfiguration = Object.freeze({
  brackets: JSON_BRACKETS,
  autoClosingPairs: JSON_PAIRS,
  surroundingPairs: JSON_SURROUNDING_PAIRS,
  indentationRules: JSON_INDENTATION_RULES,
});
const JSONC_CONFIGURATION: LanguageConfiguration = Object.freeze({
  comments: Object.freeze({
    lineComment: "//",
    blockComment: pair("/*", "*/"),
  }),
  brackets: JSON_BRACKETS,
  autoClosingPairs: JSON_PAIRS,
  surroundingPairs: JSON_SURROUNDING_PAIRS,
  indentationRules: JSON_INDENTATION_RULES,
  onEnterRules: JSONC_ON_ENTER_RULES,
});

/** Registers Alpha's built-in editing rules into one caller-owned realm. */
export function registerAlphaBuiltinLanguageConfigurations(registry: LanguageConfigurationRegistry): IDisposable {
  if (!(registry instanceof LanguageConfigurationRegistry)) {
    throw new TypeError("Built-in language configurations require a language configuration registry");
  }
  const registrations = new DisposableStore();
  for (const languageId of ECMASCRIPT_LANGUAGE_IDS) registrations.add(registry.register(languageId, ECMASCRIPT_CONFIGURATION));
  registrations.add(registry.register("json", JSON_CONFIGURATION));
  registrations.add(registry.register("jsonc", JSONC_CONFIGURATION));
  return registrations;
}

/** Resolves built-in rules for isolated providers that receive no registry. */
export function createAlphaBuiltinLanguageConfigurationSource(): LanguageConfigurationSource {
  const configurations = new Map<string, ResolvedLanguageConfiguration>();
  for (const languageId of ECMASCRIPT_LANGUAGE_IDS) configurations.set(languageId, resolved(languageId, ECMASCRIPT_CONFIGURATION));
  configurations.set("json", resolved("json", JSON_CONFIGURATION));
  configurations.set("jsonc", resolved("jsonc", JSONC_CONFIGURATION));
  return Object.freeze({
    getLanguageConfiguration(languageId: string): ResolvedLanguageConfiguration {
      assertLanguageId(languageId);
      return configurations.get(languageId) ?? resolved(languageId, {});
    },
  });
}

function pair(open: string, close: string): LanguageCharacterPair {
  return Object.freeze({ open, close });
}

function autoPair(open: string, close: string, notIn: readonly LanguageAutoClosingTokenContext[]): LanguageAutoClosingPair {
  return Object.freeze({ open, close, notIn: Object.freeze([...notIn]) });
}

function resolved(languageId: string, configuration: LanguageConfiguration): ResolvedLanguageConfiguration {
  const comments = configuration.comments && {
    ...(configuration.comments.lineComment ? { lineComment: configuration.comments.lineComment } : {}),
    ...(configuration.comments.blockComment ? { blockComment: configuration.comments.blockComment } : {}),
  };
  return Object.freeze({
    languageId,
    revision: 1,
    comments: Object.freeze(comments ?? {}),
    brackets: configuration.brackets ?? Object.freeze([]),
    autoClosingPairs: configuration.autoClosingPairs ?? configuration.brackets ?? Object.freeze([]),
    surroundingPairs: configuration.surroundingPairs ?? Object.freeze((configuration.autoClosingPairs ?? configuration.brackets ?? []).map(value => pair(value.open, value.close))),
    autoCloseBefore: configuration.autoCloseBefore ?? DEFAULT_LANGUAGE_AUTO_CLOSE_BEFORE,
    ...(configuration.indentationRules ? { indentationRules: copyIndentationRules(configuration.indentationRules) } : {}),
    onEnterRules: Object.freeze((configuration.onEnterRules ?? []).map(copyOnEnterRule)),
  });
}

function onEnter(beforeText: RegExp, indentAction: LanguageIndentAction, options: BuiltinOnEnterOptions = {}): LanguageOnEnterRule {
  const { afterText, previousLineText, appendText, removeText } = options;
  return Object.freeze({
    beforeText,
    ...(afterText === undefined ? {} : { afterText }),
    ...(previousLineText === undefined ? {} : { previousLineText }),
    action: Object.freeze({
      indentAction,
      ...(appendText === undefined ? {} : { appendText }),
      ...(removeText === undefined ? {} : { removeText }),
    }),
  });
}

function copyIndentationRules(rules: LanguageIndentationRules): LanguageIndentationRules {
  return Object.freeze({
    decreaseIndentPattern: copyPattern(rules.decreaseIndentPattern),
    increaseIndentPattern: copyPattern(rules.increaseIndentPattern),
    ...(rules.indentNextLinePattern === undefined ? {} : {
      indentNextLinePattern: rules.indentNextLinePattern === null ? null : copyPattern(rules.indentNextLinePattern),
    }),
    ...(rules.unIndentedLinePattern === undefined ? {} : {
      unIndentedLinePattern: rules.unIndentedLinePattern === null ? null : copyPattern(rules.unIndentedLinePattern),
    }),
  });
}

function copyOnEnterRule(rule: LanguageOnEnterRule): LanguageOnEnterRule {
  return onEnter(copyPattern(rule.beforeText), rule.action.indentAction, {
    ...(rule.afterText === undefined ? {} : { afterText: copyPattern(rule.afterText) }),
    ...(rule.previousLineText === undefined ? {} : { previousLineText: copyPattern(rule.previousLineText) }),
    ...(rule.action.appendText === undefined ? {} : { appendText: rule.action.appendText }),
    ...(rule.action.removeText === undefined ? {} : { removeText: rule.action.removeText }),
  });
}

function copyPattern(pattern: RegExp): RegExp {
  return Object.freeze(new RegExp(pattern.source, pattern.flags));
}

import { strict as assert } from "node:assert";
import test from "node:test";
import { createAlphaBuiltinLanguageConfigurationSource } from "../../common/languageBuiltinConfigurations.js";
import { LanguageConfigurationRegistry, LanguageIndentAction } from "../../common/languageConfiguration.js";

test("Language configurations compose by field, priority, and registration order", () => {
  using registry = new LanguageConfigurationRegistry();
  const revisions: number[] = [];
  using listener = registry.onDidChangeConfiguration(event => revisions.push(event.configuration.revision));
  using base = registry.register("typescript", {
    comments: {
      lineComment: "//",
      blockComment: { open: "/*", close: "*/" },
    },
    brackets: [
      { open: "(", close: ")" },
      { open: "{", close: "}" },
    ],
  });
  const override = registry.register("typescript", {
    comments: { lineComment: "#" },
  }, { priority: 10 });

  const composed = registry.getLanguageConfiguration("typescript");
  assert.equal(composed.comments.lineComment, "#");
  assert.deepEqual(composed.comments.blockComment, { open: "/*", close: "*/" });
  assert.deepEqual(composed.brackets, [{ open: "(", close: ")" }, { open: "{", close: "}" }]);
  assert.equal(Object.isFrozen(composed), true);
  assert.equal(Object.isFrozen(composed.brackets), true);

  override.dispose();
  const restored = registry.getLanguageConfiguration("typescript");
  assert.equal(restored.comments.lineComment, "//");
  assert.equal(restored.revision, 3);
  assert.deepEqual(revisions, [1, 2, 3]);
});

test("Language configuration contributions may explicitly clear inherited fields", () => {
  using registry = new LanguageConfigurationRegistry();
  using base = registry.register("json", {
    comments: { lineComment: "//" },
    brackets: [{ open: "{", close: "}" }],
  });
  const clearing = registry.register("json", {
    comments: null,
    brackets: null,
  }, { priority: 1 });

  assert.deepEqual(registry.getLanguageConfiguration("json").comments, {});
  assert.deepEqual(registry.getLanguageConfiguration("json").brackets, []);

  clearing.dispose();
  assert.equal(registry.getLanguageConfiguration("json").comments.lineComment, "//");
  assert.deepEqual(registry.getLanguageConfiguration("json").brackets, [{ open: "{", close: "}" }]);
});

test("Language configuration validation is atomic and identities stay language-owned", () => {
  using registry = new LanguageConfigurationRegistry();
  const initial = registry.getLanguageConfiguration("plaintext");

  assert.throws(() => registry.register("plaintext", {
    comments: { lineComment: "" },
  }), /non-empty/);
  assert.throws(() => registry.register("plaintext", {
    brackets: [
      { open: "(", close: ")" },
      { open: ")", close: "}" },
    ],
  }), /unambiguous/);
  assert.throws(() => registry.register("*", {}), /Language ID/);

  assert.equal(registry.getLanguageConfiguration("plaintext"), initial);
  assert.equal(initial.revision, 0);
});

test("Language configuration registration and registry lifecycles are independent", () => {
  const registry = new LanguageConfigurationRegistry();
  const registration = registry.register("typescript", {
    comments: { lineComment: "//" },
  });

  registration.dispose();
  registration.dispose();
  assert.deepEqual(registry.getLanguageConfiguration("typescript").comments, {});

  registry.dispose();
  assert.throws(() => registry.getLanguageConfiguration("typescript"), /already disposed/);
  registration.dispose();
});

test("Auto-closing and surrounding pairs fall back canonically and accept quote pairs", () => {
  using registry = new LanguageConfigurationRegistry();
  using brackets = registry.register("demo", {
    brackets: [{ open: "(", close: ")" }],
  });

  const fallback = registry.getLanguageConfiguration("demo");
  assert.deepEqual(fallback.autoClosingPairs, [{ open: "(", close: ")" }]);
  assert.deepEqual(fallback.surroundingPairs, [{ open: "(", close: ")" }]);

  const quotes = registry.register("demo", {
    autoClosingPairs: [{ open: "\"", close: "\"" }],
    autoCloseBefore: " ",
  }, { priority: 1 });
  const overridden = registry.getLanguageConfiguration("demo");
  assert.deepEqual(overridden.autoClosingPairs, [{ open: "\"", close: "\"" }]);
  assert.deepEqual(overridden.surroundingPairs, [{ open: "\"", close: "\"" }]);
  assert.equal(overridden.autoCloseBefore, " ");

  quotes.dispose();
  assert.deepEqual(registry.getLanguageConfiguration("demo").autoClosingPairs, [{ open: "(", close: ")" }]);
  assert.throws(() => registry.register("demo", {
    autoClosingPairs: [
      { open: "<", close: ">" },
      { open: "<", close: "/>" },
    ],
  }), /open tokens must be unique/);
});

test("Auto-closing token exclusions are immutable and validate their closed vocabulary", () => {
  using registry = new LanguageConfigurationRegistry();
  using pairs = registry.register("demo", {
    autoClosingPairs: [{
      open: "\"",
      close: "\"",
      notIn: ["string", "comment"],
    }],
  });
  const pair = registry.getLanguageConfiguration("demo").autoClosingPairs[0]!;
  assert.deepEqual(pair, { open: "\"", close: "\"", notIn: ["string", "comment"] });
  assert.equal(Object.isFrozen(pair.notIn), true);

  assert.throws(() => registry.register("demo", {
    autoClosingPairs: [{
      open: "'",
      close: "'",
      notIn: ["regex" as "string"],
    }],
  }), /string or comment/);
  assert.throws(() => registry.register("demo", {
    autoClosingPairs: [{
      open: "'",
      close: "'",
      notIn: ["string", "string"],
    }],
  }), /must be unique/);
});

test("Indentation and on-enter rules compose, clone, clear, and restore atomically", () => {
  using registry = new LanguageConfigurationRegistry();
  const increase = /\{$/g;
  using base = registry.register("demo", {
    indentationRules: {
      increaseIndentPattern: increase,
      decreaseIndentPattern: /^\s*\}/,
    },
    onEnterRules: [{
      beforeText: /\/\*\*$/,
      afterText: /^\s*\*\//,
      action: {
        indentAction: LanguageIndentAction.IndentOutdent,
        appendText: " * ",
      },
    }],
  });
  const resolved = registry.getLanguageConfiguration("demo");
  assert.notEqual(resolved.indentationRules?.increaseIndentPattern, increase);
  assert.equal(resolved.indentationRules?.increaseIndentPattern.source, "\\{$");
  assert.equal(Object.isFrozen(resolved.indentationRules?.increaseIndentPattern), true);
  assert.equal(Object.isFrozen(resolved.onEnterRules), true);
  assert.equal(Object.isFrozen(resolved.onEnterRules[0]?.action), true);

  const clearing = registry.register("demo", {
    indentationRules: null,
    onEnterRules: null,
  }, { priority: 1 });
  assert.equal(registry.getLanguageConfiguration("demo").indentationRules, undefined);
  assert.deepEqual(registry.getLanguageConfiguration("demo").onEnterRules, []);

  clearing.dispose();
  assert.equal(registry.getLanguageConfiguration("demo").indentationRules?.increaseIndentPattern.source, "\\{$");
  assert.equal(registry.getLanguageConfiguration("demo").onEnterRules.length, 1);
});

test("Indentation and on-enter configuration rejects ambiguous values before registration", () => {
  using registry = new LanguageConfigurationRegistry();
  const initial = registry.getLanguageConfiguration("demo");

  assert.throws(() => registry.register("demo", {
    indentationRules: {
      increaseIndentPattern: "{" as unknown as RegExp,
      decreaseIndentPattern: /}/,
    },
  }), /RegExp/);
  assert.throws(() => registry.register("demo", {
    onEnterRules: [{
      beforeText: /x/,
      action: { indentAction: "sideways" as LanguageIndentAction },
    }],
  }), /unknown indentation action/);
  assert.throws(() => registry.register("demo", {
    onEnterRules: [{
      beforeText: /x/,
      action: { indentAction: LanguageIndentAction.None, appendText: "\n" },
    }],
  }), /single-line/);
  assert.throws(() => registry.register("demo", {
    onEnterRules: [{
      beforeText: /x/,
      action: { indentAction: LanguageIndentAction.None, removeText: -1 },
    }],
  }), /non-negative/);

  assert.equal(registry.getLanguageConfiguration("demo"), initial);
});

test("Isolated built-in sources expose immutable Enter and indentation rules", () => {
  const source = createAlphaBuiltinLanguageConfigurationSource();
  const typescript = source.getLanguageConfiguration("typescript");
  const json = source.getLanguageConfiguration("json");

  assert.ok(typescript.indentationRules);
  assert.equal(typescript.onEnterRules.length > 0, true);
  assert.equal(Object.isFrozen(typescript.indentationRules), true);
  assert.equal(Object.isFrozen(typescript.indentationRules.increaseIndentPattern), true);
  assert.equal(Object.isFrozen(typescript.onEnterRules), true);
  assert.equal(Object.isFrozen(typescript.onEnterRules[0]?.beforeText), true);
  assert.deepEqual(typescript.autoClosingPairs.find(pair => pair.open === "'")?.notIn, ["string", "comment"]);
  assert.equal("notIn" in typescript.surroundingPairs.find(pair => pair.open === "'")!, false);
  assert.equal(json.onEnterRules.length, 0);
  assert.ok(json.indentationRules);
});

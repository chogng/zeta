import { Emitter } from "../../../base/common/event.js";
import { DisposableOwner, } from "../../../base/common/lifecycle.js";
import { createServiceIdentifier, } from "../../instantiation/common/instantiation.js";
export const IContextKeyService = createServiceIdentifier("contextKeyService");
/**
 * Declares one context key and its default independently of a concrete scope.
 */
export class RawContextKey {
    key;
    defaultValue;
    constructor(key, defaultValue) {
        this.key = key;
        this.defaultValue = defaultValue;
        if (!key)
            throw new TypeError("Context key must not be empty");
    }
    bindTo(service) {
        return service.createKey(this.key, this.defaultValue);
    }
    isEqualTo(value) {
        return ContextKeyExpr.equals(this.key, value);
    }
}
class Expression {
    evaluate;
    keys;
    constructor(evaluate, keys) {
        this.evaluate = evaluate;
        this.keys = keys;
    }
}
/** Factory functions for context expressions used by contributions. */
export const ContextKeyExpr = {
    has(key) {
        return new Expression((context) => Boolean(context.getValue(key)), () => new Set([key]));
    },
    not(key) {
        return new Expression((context) => !context.getValue(key), () => new Set([key]));
    },
    equals(key, value) {
        return new Expression((context) => Object.is(context.getValue(key), value), () => new Set([key]));
    },
    notEquals(key, value) {
        return new Expression((context) => !Object.is(context.getValue(key), value), () => new Set([key]));
    },
    and(...expressions) {
        const defined = expressions.filter((expression) => Boolean(expression));
        if (defined.length === 0)
            return undefined;
        return combineExpressions(defined, (context) => defined.every((expression) => expression.evaluate(context)));
    },
    or(...expressions) {
        const defined = expressions.filter((expression) => Boolean(expression));
        if (defined.length === 0)
            return undefined;
        return combineExpressions(defined, (context) => defined.some((expression) => expression.evaluate(context)));
    },
};
class AbstractContextKeyService extends DisposableOwner {
    #values = new Map();
    #parent;
    state;
    constructor(state, parent) {
        super();
        this.state = state;
        this.#parent = parent;
        this.defer(() => {
            const keys = new Set(this.#values.keys());
            this.#values.clear();
            fireContextChange(this.state, keys);
        });
    }
    get onDidChangeContext() {
        return this.state.emitter.event;
    }
    getValue(key) {
        if (this.#values.has(key)) {
            return this.#values.get(key);
        }
        return this.#parent?.getValue(key);
    }
    contextMatchesRules(expression, target) {
        return expression?.evaluate(this.getContext(target)) ?? true;
    }
    createKey(key, defaultValue) {
        return new BoundContextKey(this, key, defaultValue);
    }
    createScoped(target) {
        const parent = findScopedContext(this.state, getComposedParent(target)) ?? this;
        return new ScopedContextKeyService(parent, target, this.state);
    }
    getContext(target) {
        if (!target)
            return this;
        return findScopedContext(this.state, target) ??
            this.state.root ??
            this;
    }
    setContext(key, value) {
        if (this.#values.has(key) && Object.is(this.#values.get(key), value)) {
            return;
        }
        this.#values.set(key, value);
        fireContextChange(this.state, new Set([key]));
    }
    removeContext(key) {
        if (!this.#values.delete(key))
            return;
        fireContextChange(this.state, new Set([key]));
    }
}
/** Default mutable context key service for one workbench window. */
export class ContextKeyService extends AbstractContextKeyService {
    constructor() {
        const state = {
            emitter: new Emitter(),
            scopes: new WeakMap(),
            root: undefined,
        };
        super(state);
        state.root = this;
        this.own(state.emitter);
    }
}
class ScopedContextKeyService extends AbstractContextKeyService {
    constructor(parent, target, state) {
        if (state.scopes.has(target)) {
            throw new Error("A context key scope is already bound to this element");
        }
        super(state, parent);
        state.scopes.set(target, this);
        this.defer(() => {
            if (state.scopes.get(target) === this)
                state.scopes.delete(target);
        });
    }
}
class BoundContextKey {
    #service;
    #key;
    #defaultValue;
    constructor(service, key, defaultValue) {
        if (!key)
            throw new TypeError("Context key must not be empty");
        this.#service = service;
        this.#key = key;
        this.#defaultValue = defaultValue;
        this.reset();
    }
    set(value) {
        this.#service.setContext(this.#key, value);
    }
    reset() {
        if (this.#defaultValue === undefined) {
            this.#service.removeContext(this.#key);
        }
        else {
            this.#service.setContext(this.#key, this.#defaultValue);
        }
    }
    get() {
        return this.#service.getValue(this.#key);
    }
}
function combineExpressions(expressions, evaluate) {
    const keys = new Set();
    for (const expression of expressions) {
        for (const key of expression.keys())
            keys.add(key);
    }
    return new Expression(evaluate, () => keys);
}
function fireContextChange(state, keys) {
    if (keys.size === 0)
        return;
    state.emitter.fire({
        keys,
        affectsSome(candidateKeys) {
            for (const key of keys) {
                if (candidateKeys.has(key))
                    return true;
            }
            return false;
        },
    });
}
function getComposedParent(node) {
    if (node.parentNode)
        return node.parentNode;
    const root = node.getRootNode();
    return "host" in root ? root.host : null;
}
function findScopedContext(state, target) {
    for (let current = target; current; current = getComposedParent(current)) {
        const scoped = state.scopes.get(current);
        if (scoped)
            return scoped;
    }
    return undefined;
}

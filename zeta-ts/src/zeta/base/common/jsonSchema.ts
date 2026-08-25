import { type JsonDocument, type JsonValueNode } from './json.js';
import type { JsonValue } from './jsonValue.js';

export type JsonSchemaType = 'array' | 'boolean' | 'integer' | 'null' | 'number' | 'object' | 'string';

export interface JsonSchema {
	readonly id?: string;
	readonly title?: string;
	readonly description?: string;
	readonly type?: JsonSchemaType | readonly JsonSchemaType[];
	readonly default?: JsonValue;
	readonly enum?: readonly JsonValue[];
	readonly enumDescriptions?: readonly string[];
	readonly properties?: Readonly<Record<string, JsonSchema>>;
	readonly required?: readonly string[];
	readonly additionalProperties?: boolean | JsonSchema;
	readonly items?: JsonSchema;
	readonly minimum?: number;
	readonly maximum?: number;
}

export interface JsonSchemaIssue {
	readonly message: string;
	readonly offset: number;
	readonly length: number;
}

/** Resolves a nested schema using object-property and array-index path segments. */
export function jsonSchemaAtPath(schema: JsonSchema | undefined, path: readonly (string | number)[]): JsonSchema | undefined {
	let current = schema;
	for (const segment of path) {
		if (!current) return undefined;
		if (typeof segment === 'number') {
			current = current.items;
			continue;
		}
		current = current.properties?.[segment] ?? (typeof current.additionalProperties === 'object' ? current.additionalProperties : undefined);
	}
	return current;
}

/** Validates a parsed JSON document against the supported structural schema vocabulary. */
export function validateJsonSchema(document: JsonDocument, schema: JsonSchema | undefined): readonly JsonSchemaIssue[] {
	if (!document.root || !schema) return Object.freeze([]);
	const issues: JsonSchemaIssue[] = [];
	validateNode(document.root, schema, issues);
	return Object.freeze(issues);
}

function validateNode(node: JsonValueNode, schema: JsonSchema, issues: JsonSchemaIssue[]): void {
	const types = schema.type === undefined ? undefined : Array.isArray(schema.type) ? schema.type : [schema.type];
	if (types && !types.some(type => nodeMatchesType(node, type))) {
		issues.push(issue(`Expected ${types.join(' or ')}`, node));
		return;
	}
	if (schema.enum && !schema.enum.some(candidate => equalJson(candidate, nodeValue(node)))) {
		issues.push(issue('Value is not one of the permitted values', node));
	}
	if (node.type === 'number') {
		if (schema.minimum !== undefined && node.value < schema.minimum) issues.push(issue(`Value must be at least ${schema.minimum}`, node));
		if (schema.maximum !== undefined && node.value > schema.maximum) issues.push(issue(`Value must be at most ${schema.maximum}`, node));
	}
	if (node.type === 'array' && schema.items) {
		for (const item of node.items) validateNode(item, schema.items, issues);
	}
	if (node.type !== 'object') return;
	const seen = new Set<string>();
	for (const property of node.properties) {
		if (seen.has(property.key)) issues.push(issue(`Duplicate property '${property.key}'`, property.keyNode));
		seen.add(property.key);
		if (!property.valueNode) continue;
		const propertySchema = schema.properties?.[property.key];
		if (propertySchema) {
			validateNode(property.valueNode, propertySchema, issues);
			continue;
		}
		if (schema.additionalProperties === false) issues.push(issue(`Property '${property.key}' is not permitted`, property.keyNode));
		if (typeof schema.additionalProperties === 'object') validateNode(property.valueNode, schema.additionalProperties, issues);
	}
	for (const required of schema.required ?? []) {
		if (!seen.has(required)) issues.push(issue(`Required property '${required}' is missing`, node));
	}
}

function nodeMatchesType(node: JsonValueNode, type: JsonSchemaType): boolean {
	if (type === 'integer') return node.type === 'number' && Number.isInteger(node.value);
	return node.type === type;
}

function nodeValue(node: JsonValueNode): JsonValue {
	switch (node.type) {
		case 'string': return node.value;
		case 'number': return node.value;
		case 'boolean': return node.value;
		case 'null': return null;
		case 'array': return node.items.map(nodeValue);
		case 'object': {
			const value: Record<string, JsonValue> = {};
			for (const property of node.properties) {
				if (property.valueNode) Object.defineProperty(value, property.key, { value: nodeValue(property.valueNode), enumerable: true, configurable: true, writable: true });
			}
			return value;
		}
	}
}

function equalJson(left: JsonValue, right: JsonValue): boolean {
	return JSON.stringify(left) === JSON.stringify(right);
}

function issue(message: string, node: { readonly offset: number; readonly length: number }): JsonSchemaIssue {
	return Object.freeze({ message, offset: node.offset, length: Math.max(node.length, 1) });
}

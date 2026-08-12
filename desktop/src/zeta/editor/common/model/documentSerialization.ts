import { createDocumentNode, type DocumentAttributeValue, type DocumentAttributes, type DocumentMark, type DocumentNode } from "./document.js";
import { type DocumentSchema, type DocumentValidationOptions } from "./documentSchema.js";

export const DOCUMENT_SERIALIZATION_FORMAT = "zeta.document";
export const DOCUMENT_SERIALIZATION_VERSION = 1;
export const DOCUMENT_FRAGMENT_SERIALIZATION_FORMAT = "zeta.document.fragment";
export const DOCUMENT_FRAGMENT_SERIALIZATION_VERSION = 1;
export const DOCUMENT_FRAGMENT_CLIPBOARD_MIME = "application/vnd.zeta.document.fragment+json";

/** JSON-safe representation of one Aster node, including incomplete transaction fragments. */
export interface SerializedDocumentNode {
  readonly id: string;
  readonly type: string;
  readonly attrs: DocumentAttributes;
  readonly content: readonly SerializedDocumentNode[];
  readonly marks: readonly DocumentMark[];
  readonly text?: string;
}

export interface DocumentFragment {
  readonly content: readonly DocumentNode[];
}

export class DocumentSerializationError extends Error {
  constructor(message: string, options?: ErrorOptions) {
    super(message, options);
    this.name = "DocumentSerializationError";
  }
}

/** Encodes a schema-valid node for transport inside a transaction or fragment. */
export function encodeDocumentNode(node: DocumentNode, schema: DocumentSchema, options: DocumentValidationOptions = {}): SerializedDocumentNode {
  try {
    schema.validateFragment(node, options);
  } catch (error) {
    throw new DocumentSerializationError("Document node failed schema validation", { cause: error });
  }
  return encodeNode(node);
}

/** Decodes and validates one untrusted node value from a Aster transport payload. */
export function decodeDocumentNode(value: unknown, schema: DocumentSchema, options: DocumentValidationOptions = {}): DocumentNode {
  try {
    const node = decodeNode(value);
    schema.validateFragment(node, options);
    return node;
  } catch (error) {
    if (error instanceof DocumentSerializationError) throw error;
    throw new DocumentSerializationError("Document node failed schema validation", { cause: error });
  }
}

/** Serializes a validated document in a versioned envelope for persistence. */
export function serializeDocument(document: DocumentNode, schema: DocumentSchema, pretty = false): string {
  schema.validate(document);
  return JSON.stringify({ format: DOCUMENT_SERIALIZATION_FORMAT, version: DOCUMENT_SERIALIZATION_VERSION, document }, undefined, pretty ? 2 : undefined);
}

/** Parses, validates, and freezes a persisted document without trusting its JSON shape. */
export function deserializeDocument(value: string | unknown, schema: DocumentSchema): DocumentNode {
  let parsed: unknown = value;
  if (typeof value === "string") {
    try {
      parsed = JSON.parse(value) as unknown;
    } catch (error) {
      throw new DocumentSerializationError("Structured document JSON is invalid", { cause: error });
    }
  }
  if (!isRecord(parsed) || parsed.format !== DOCUMENT_SERIALIZATION_FORMAT || parsed.version !== DOCUMENT_SERIALIZATION_VERSION) throw new DocumentSerializationError("Unsupported structured document format or version");
  try {
    const document = decodeNode(parsed.document);
    schema.validate(document);
    return document;
  } catch (error) {
    if (error instanceof DocumentSerializationError) throw error;
    throw new DocumentSerializationError("Structured document failed schema validation", { cause: error });
  }
}

/** Serializes a validated selection fragment for Aster-aware clipboard transport. */
export function serializeDocumentFragment(fragment: DocumentFragment, schema: DocumentSchema, pretty = false): string {
  validateFragmentContent(fragment.content, schema);
  return JSON.stringify({ format: DOCUMENT_FRAGMENT_SERIALIZATION_FORMAT, version: DOCUMENT_FRAGMENT_SERIALIZATION_VERSION, content: fragment.content }, undefined, pretty ? 2 : undefined);
}

/** Parses and validates a Aster clipboard fragment without trusting its JSON shape. */
export function deserializeDocumentFragment(value: string | unknown, schema: DocumentSchema): DocumentFragment {
  let parsed: unknown = value;
  if (typeof value === "string") {
    try {
      parsed = JSON.parse(value) as unknown;
    } catch (error) {
      throw new DocumentSerializationError("Structured document fragment JSON is invalid", { cause: error });
    }
  }
  if (!isRecord(parsed) || parsed.format !== DOCUMENT_FRAGMENT_SERIALIZATION_FORMAT || parsed.version !== DOCUMENT_FRAGMENT_SERIALIZATION_VERSION || !Array.isArray(parsed.content)) throw new DocumentSerializationError("Unsupported structured document fragment format or version");
  try {
    const content = Object.freeze(parsed.content.map(decodeNode));
    validateFragmentContent(content, schema);
    return Object.freeze({ content });
  } catch (error) {
    if (error instanceof DocumentSerializationError) throw error;
    throw new DocumentSerializationError("Structured document fragment failed schema validation", { cause: error });
  }
}

function decodeNode(value: unknown): DocumentNode {
  if (!isRecord(value) || typeof value.id !== "string" || typeof value.type !== "string") throw new DocumentSerializationError("Serialized document node is invalid");
  if (!Array.isArray(value.content)) throw new DocumentSerializationError(`Serialized node '${value.id}' must contain an array`);
  const attrs = decodeAttributes(value.attrs, `node:${value.id}`);
  const marks = value.marks === undefined ? [] : decodeMarks(value.marks, value.id);
  if (value.text !== undefined && typeof value.text !== "string") throw new DocumentSerializationError(`Serialized text for '${value.id}' is invalid`);
  return createDocumentNode({ id: value.id, type: value.type, attrs, content: value.content.map(decodeNode), marks, ...(value.text === undefined ? {} : { text: value.text }) });
}

function encodeNode(node: DocumentNode): SerializedDocumentNode {
  return {
    id: node.id,
    type: node.type,
    attrs: { ...node.attrs },
    content: node.content.map(encodeNode),
    marks: node.marks.map(mark => ({ type: mark.type, attrs: { ...mark.attrs } })),
    ...(node.text === undefined ? {} : { text: node.text }),
  };
}

function validateFragmentContent(content: readonly DocumentNode[], schema: DocumentSchema): void {
  let rootId = "__zeta_document_fragment_root__";
  while (content.some(node => node.id === rootId)) rootId += "_";
  schema.createDocument(content, rootId);
}

function decodeMarks(value: unknown, nodeId: string): readonly DocumentMark[] {
  if (!Array.isArray(value)) throw new DocumentSerializationError(`Serialized marks for '${nodeId}' must be an array`);
  return value.map(mark => {
    if (!isRecord(mark) || typeof mark.type !== "string") throw new DocumentSerializationError(`Serialized mark on '${nodeId}' is invalid`);
    return Object.freeze({ type: mark.type, attrs: decodeAttributes(mark.attrs, `mark:${mark.type}`) });
  });
}

function decodeAttributes(value: unknown, owner: string): DocumentAttributes {
  if (!isRecord(value)) throw new DocumentSerializationError(`Serialized attributes for '${owner}' are invalid`);
  const attrs: Record<string, DocumentAttributeValue> = {};
  for (const [key, attribute] of Object.entries(value)) {
    if (!isAttributeValue(attribute)) throw new DocumentSerializationError(`Serialized attribute '${key}' on '${owner}' is invalid`);
    attrs[key] = attribute;
  }
  return Object.freeze(attrs);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isAttributeValue(value: unknown): value is DocumentAttributeValue {
  return value === null || typeof value === "string" || typeof value === "boolean" || (typeof value === "number" && Number.isFinite(value));
}

import { APP_SERVER_PROTOCOL_MAJOR, APP_SERVER_SCHEMA_HASH, type InitializeResult, type ServerCapabilities } from '../../../../../generated/app-server/types.js';
import { isRecord } from '../../../base/common/types.js';

const serverCapabilityFields = [
	'agentInteractions',
	'documentCollaboration',
	'sessions',
	'threads',
	'turns',
	'resources',
	'attachments',
	'fileSystem',
	'git',
	'contentSearch',
	'codebase',
	'cloudCodebase',
	'terminal',
	'debugAdapter',
	'typst',
	'updateReplay',
	'extensions',
	'extensionHost',
	'connectors',
	'plugins',
	'marketplace',
	'mcp',
	'mcpOAuth',
] as const satisfies readonly (keyof ServerCapabilities)[];

export interface AppServerCapabilityRequirement {
	readonly name: string;
	readonly minVersion: number;
	readonly maxVersion: number;
}

export const requiredSessionCapabilities: readonly AppServerCapabilityRequirement[] = [
	{ name: 'sessions', minVersion: 1, maxVersion: 1 },
	{ name: 'threads', minVersion: 1, maxVersion: 1 },
	{ name: 'turns', minVersion: 1, maxVersion: 1 },
];

export type AppServerProtocolIncompatibility =
	| { readonly kind: 'missingProtocolVersion'; readonly expected: number }
	| { readonly kind: 'majorVersion'; readonly expected: number; readonly received: number }
	| { readonly kind: 'missingCapability'; readonly name: string; readonly minVersion: number; readonly maxVersion: number }
	| { readonly kind: 'capabilityVersion'; readonly name: string; readonly minVersion: number; readonly maxVersion: number; readonly received: number };

/** Initialization failure that a host may recover by selecting another trusted runtime. */
export class AppServerProtocolIncompatibleError extends Error {
	public constructor(public readonly incompatibility: AppServerProtocolIncompatibility) {
		super(describeIncompatibility(incompatibility));
		this.name = 'AppServerProtocolIncompatibleError';
	}
}

export interface AppServerProtocolDiagnostics {
	readonly clientProtocolMajor: number;
	readonly serverProtocolMajor: number;
	readonly serverProtocolRevision: number;
	readonly clientSchemaHash: string;
	readonly serverSchemaHash: string;
	readonly schemaMatches: boolean;
}

export interface ValidateAppServerInitializeOptions {
	readonly expectedServerName?: string;
	readonly requiredCapabilities?: readonly AppServerCapabilityRequirement[];
}

export function validateAppServerInitializeResult(value: unknown, options: ValidateAppServerInitializeOptions = {}): InitializeResult {
	if (!isRecord(value)) throw new Error('App Server initialize result is malformed');
	const serverInfo = value.serverInfo;
	const protocolVersion = value.protocolVersion;
	const capabilities = value.capabilities;
	if (
		!isRecord(serverInfo)
		|| typeof serverInfo.name !== 'string'
		|| serverInfo.name.trim().length === 0
		|| typeof serverInfo.version !== 'string'
		|| serverInfo.version.trim().length === 0
		|| typeof value.schemaHash !== 'string'
		|| value.schemaHash.trim().length === 0
		|| !isRecord(capabilities)
		|| serverCapabilityFields.some(field => typeof capabilities[field] !== 'boolean')
		|| !Array.isArray(value.slashCommands)
		|| value.slashCommands.some(command => !validSlashCommand(command))
	) {
		throw new Error('App Server initialize result is malformed');
	}
	if (!isRecord(protocolVersion) || !positiveSafeInteger(protocolVersion.major) || !positiveSafeInteger(protocolVersion.revision)) {
		throw new AppServerProtocolIncompatibleError({ kind: 'missingProtocolVersion', expected: APP_SERVER_PROTOCOL_MAJOR });
	}
	const contracts = capabilities.contracts;
	if (contracts !== undefined && (!isRecord(contracts) || Object.values(contracts).some(contract => !isRecord(contract) || !positiveSafeInteger(contract.version)))) {
		throw new Error('App Server initialize result is malformed');
	}
	const advertisedContracts = isRecord(contracts) ? contracts : {};
	if (options.expectedServerName && serverInfo.name !== options.expectedServerName) {
		throw new Error(`Unexpected App Server identity: ${serverInfo.name}`);
	}
	if (protocolVersion.major !== APP_SERVER_PROTOCOL_MAJOR) {
		throw new AppServerProtocolIncompatibleError({ kind: 'majorVersion', expected: APP_SERVER_PROTOCOL_MAJOR, received: protocolVersion.major });
	}
	for (const requirement of options.requiredCapabilities ?? requiredSessionCapabilities) {
		const booleanField = serverCapabilityFields.find(field => field === requirement.name);
		if (booleanField && capabilities[booleanField] !== true) {
			throw new AppServerProtocolIncompatibleError({ kind: 'missingCapability', ...requirement });
		}
		const contract = advertisedContracts[requirement.name];
		if (!isRecord(contract)) {
			throw new AppServerProtocolIncompatibleError({ kind: 'missingCapability', ...requirement });
		}
		const version = contract.version as number;
		if (version < requirement.minVersion || version > requirement.maxVersion) {
			throw new AppServerProtocolIncompatibleError({ kind: 'capabilityVersion', ...requirement, received: version });
		}
	}
	if (!isRecord(contracts)) throw new Error('App Server initialize result is malformed');
	return value as unknown as InitializeResult;
}

export function appServerProtocolDiagnostics(initialized: InitializeResult): AppServerProtocolDiagnostics {
	return {
		clientProtocolMajor: APP_SERVER_PROTOCOL_MAJOR,
		serverProtocolMajor: initialized.protocolVersion.major,
		serverProtocolRevision: initialized.protocolVersion.revision,
		clientSchemaHash: APP_SERVER_SCHEMA_HASH,
		serverSchemaHash: initialized.schemaHash,
		schemaMatches: initialized.schemaHash === APP_SERVER_SCHEMA_HASH,
	};
}

function describeIncompatibility(incompatibility: AppServerProtocolIncompatibility): string {
	switch (incompatibility.kind) {
		case 'missingProtocolVersion':
			return `Zeta App Server does not advertise a protocol version; Desktop requires major ${incompatibility.expected}`;
		case 'majorVersion':
			return `Zeta App Server protocol major mismatch: Desktop requires ${incompatibility.expected}, server advertised ${incompatibility.received}`;
		case 'missingCapability':
			return `Zeta App Server is missing required capability ${incompatibility.name}; Desktop supports versions ${incompatibility.minVersion}-${incompatibility.maxVersion}`;
		case 'capabilityVersion':
			return `Zeta App Server capability ${incompatibility.name} is incompatible: Desktop supports versions ${incompatibility.minVersion}-${incompatibility.maxVersion}, server advertised ${incompatibility.received}`;
	}
}

function positiveSafeInteger(value: unknown): value is number {
	return typeof value === 'number' && Number.isSafeInteger(value) && value > 0;
}

function validSlashCommand(value: unknown): boolean {
	return isRecord(value)
		&& typeof value.name === 'string'
		&& typeof value.description === 'string'
		&& (value.argumentMode === 'none' || value.argumentMode === 'optional');
}

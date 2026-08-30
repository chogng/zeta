import type { PermissionDto } from "../../../../../generated/app-server/types.js";
import { createServiceIdentifier } from "../../instantiation/common/instantiation.js";

export interface DirPermissionsEntry {
	readonly dir: string;
	readonly path: string | undefined;
	readonly permissions: readonly PermissionDto[];
}

export interface DirPermissionsSnapshot {
	readonly revision: number;
	readonly entries: readonly DirPermissionsEntry[];
}

export interface DirPermissionsCommandResult {
	readonly revision: number;
	readonly generation: number;
	readonly disposition: "updated" | "replayed";
}

/** Frontend contract for explicit capability sets attached to directories. */
export interface IDirPermissionsService {
	list(): Promise<DirPermissionsSnapshot>;
	read(path: string): Promise<readonly PermissionDto[] | undefined>;
	set(path: string, permissions: readonly PermissionDto[], expectedRevision: number): Promise<DirPermissionsCommandResult>;
	forget(dir: string, expectedRevision: number): Promise<DirPermissionsCommandResult>;
}

export const IDirPermissionsService = createServiceIdentifier<IDirPermissionsService>("dirPermissionsService");

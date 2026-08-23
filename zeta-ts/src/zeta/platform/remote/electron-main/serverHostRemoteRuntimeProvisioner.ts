import type { RemoteRuntimeInstallProgress } from "../common/remoteRuntimeInstallProgress.js";
import { PackagedRemoteRuntimeCatalog, type RemoteRuntimeCatalogSource } from "./packagedRemoteRuntimeCatalog.js";
import { normalizeCredentialFreeSshHost, type RunServerHostRemoteCommand, runServerHostRemoteCommand, validLocalCommand } from "./serverHostRemoteCommand.js";
import { ServerHostRemoteRuntimeFetcher } from "./serverHostRemoteRuntimeFetcher.js";
import type { RemoteRuntimeInstallRequestOptions } from "./serverHostRemoteRuntimeInstaller.js";
import { ServerHostRemoteRuntimeInstaller } from "./serverHostRemoteRuntimeInstaller.js";

export interface ServerHostRemoteRuntimeProvisionerOptions {
	readonly source: RemoteRuntimeCatalogSource;
	readonly serverHostExecutable: string;
	readonly sshExecutable: string;
	readonly environment: NodeJS.ProcessEnv;
	readonly installRoot?: string;
	readonly onProgress?: (progress: RemoteRuntimeInstallProgress) => void;
	readonly runCommand?: RunServerHostRemoteCommand;
}

/** Selects a package-authenticated artifact for the probed host and delegates installation. */
export class ServerHostRemoteRuntimeProvisioner {
	private readonly runCommand: RunServerHostRemoteCommand;
	private catalog: Promise<PackagedRemoteRuntimeCatalog> | undefined;

	constructor(private readonly options: ServerHostRemoteRuntimeProvisionerOptions) {
		if (!validLocalCommand(options.serverHostExecutable) || !validLocalCommand(options.sshExecutable)) throw new Error("Remote provisioner executables must be non-empty and contain no control characters");
		this.runCommand = options.runCommand ?? runServerHostRemoteCommand;
	}

	async install(host: string, request: RemoteRuntimeInstallRequestOptions = {}): Promise<string> {
		const normalizedHost = normalizeCredentialFreeSshHost(host);
		const target = await this.probeTarget(normalizedHost, request.signal);
		const reportProgress = request.onProgress ?? this.options.onProgress;
		const artifact = this.options.source.kind === "network"
			? await new ServerHostRemoteRuntimeFetcher({ serverHostExecutable: this.options.serverHostExecutable, environment: this.options.environment, source: this.options.source, runCommand: this.runCommand }).fetch(target, { signal: request.signal, onProgress: reportProgress })
			: (await this.loadCatalog()).artifactFor(target);
		if (artifact === undefined) throw new Error(`The Desktop release has no Remote runtime for ${target}`);
		return new ServerHostRemoteRuntimeInstaller({
			serverHostExecutable: this.options.serverHostExecutable,
			sshExecutable: this.options.sshExecutable,
			environment: this.options.environment,
			artifact,
			installRoot: this.options.installRoot,
			runCommand: this.runCommand,
		}).install(normalizedHost, { signal: request.signal, onProgress: reportProgress });
	}

	private loadCatalog(): Promise<PackagedRemoteRuntimeCatalog> {
		if (this.options.source.kind !== "packaged") throw new Error("Network Remote runtime sources do not have a packaged catalog");
		this.catalog ??= PackagedRemoteRuntimeCatalog.load(this.options.source.bundleRoot, this.options.source.expectedSha256);
		return this.catalog;
	}

	private async probeTarget(host: string, signal: AbortSignal | undefined): Promise<string> {
		const result = await this.runCommand(this.options.serverHostExecutable, ["remote", "probe", "--host", host, "--ssh", this.options.sshExecutable], this.options.environment, signal === undefined ? undefined : { onStderrData: () => {}, signal });
		if (result.exitCode !== 0) {
			const diagnostic = result.stderr.trim() || result.stdout.trim() || `exit code ${result.exitCode ?? "unknown"}`;
			throw new Error(`Remote platform probe failed: ${diagnostic}`);
		}
		const targets = result.stdout.split(/\r?\n/u).map(line => line.trim()).filter(line => line.length > 0);
		if (targets.length !== 1) throw new Error("Remote platform probe did not return exactly one package target");
		return targets[0]!;
	}
}

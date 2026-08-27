import { Disposable, toDisposable, type IDisposable } from "../../../base/common/lifecycle.js";
import type { ILogServiceHost, ILogSink, LogEntry, LogLevel } from "./logService.js";

export interface LogServiceOptions {
	readonly sinks?: readonly ILogSink[];
	readonly now?: () => number;
	readonly onSinkError?: (entry: LogEntry, error: unknown) => void;
}

/** Multiplexes structured entries to runtime-selected sinks. */
export class LogService extends Disposable implements ILogServiceHost {
	private readonly sinks = new Set<ILogSink>();
	private readonly now: () => number;
	private readonly onSinkError: (entry: LogEntry, error: unknown) => void;

	constructor(options: LogServiceOptions = {}) {
		super();
		this.now = options.now ?? Date.now;
		this.onSinkError = options.onSinkError ?? reportSinkFailure;
		for (const sink of options.sinks ?? []) this.sinks.add(sink);
		this._register(toDisposable(() => this.sinks.clear()));
	}

	registerSink(sink: ILogSink): IDisposable {
		this.sinks.add(sink);
		return toDisposable(() => this.sinks.delete(sink));
	}

	trace(category: string, message: string): void { this.publish("trace", category, message); }
	debug(category: string, message: string): void { this.publish("debug", category, message); }
	info(category: string, message: string): void { this.publish("information", category, message); }
	warn(category: string, message: string, error?: unknown): void { this.publish("warning", category, message, error); }
	error(category: string, message: string, error?: unknown): void { this.publish("error", category, message, error); }

	private publish(level: LogLevel, category: string, message: string, error?: unknown): void {
		if (!category.trim()) throw new TypeError("Log category must not be empty");
		if (!message.trim()) throw new TypeError("Log message must not be empty");
		const entry: LogEntry = Object.freeze({ timestampMillis: this.now(), level, category, message, error });
		for (const sink of this.sinks) {
			try { sink.log(entry); }
			catch (sinkError) { this.onSinkError(entry, sinkError); }
		}
	}
}

function reportSinkFailure(entry: LogEntry, error: unknown): void {
	console.error(`Log sink failed while handling [${entry.category}] ${entry.message}`, error);
}

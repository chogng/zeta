import type { ILogSink, LogEntry } from "./logService.js";

/** Development and host-console projection of structured log entries. */
export class ConsoleLogSink implements ILogSink {
	constructor(private readonly target: Pick<Console, "debug" | "error" | "info" | "warn"> = console) {}

	log(entry: LogEntry): void {
		const message = `[${entry.category}] ${entry.message}`;
		const argumentsList = entry.error === undefined ? [message] : [message, entry.error];
		switch (entry.level) {
			case "trace":
			case "debug": this.target.debug(...argumentsList); break;
			case "information": this.target.info(...argumentsList); break;
			case "warning": this.target.warn(...argumentsList); break;
			case "error": this.target.error(...argumentsList); break;
		}
	}
}

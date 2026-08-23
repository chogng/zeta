import type { IDisposable } from "../../../base/common/lifecycle.js";
import { createServiceIdentifier } from "../../instantiation/common/instantiation.js";

export type LogLevel = "trace" | "debug" | "information" | "warning" | "error";

export interface LogEntry {
  readonly timestampMillis: number;
  readonly level: LogLevel;
  readonly category: string;
  readonly message: string;
  readonly error: unknown | undefined;
}

export interface ILogSink {
  log(entry: LogEntry): void;
}

/** Structured application logging independent of a console or UI sink. */
export interface ILogService {
  trace(category: string, message: string): void;
  debug(category: string, message: string): void;
  info(category: string, message: string): void;
  warn(category: string, message: string, error?: unknown): void;
  error(category: string, message: string, error?: unknown): void;
}

export interface ILogServiceHost extends ILogService {
  registerSink(sink: ILogSink): IDisposable;
}

export const ILogService = createServiceIdentifier<ILogService>("logService");

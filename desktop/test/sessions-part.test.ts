import assert from "node:assert/strict";
import test from "node:test";
import { JSDOM } from "jsdom";
import { Emitter } from "../src/zeta/base/common/event.js";
import type {
  IWorkbenchSessionService,
} from "../src/zeta/workbench/services/sessions/common/sessionService.js";
import { SessionsPart } from "../src/zeta/sessions/browser/parts/sessionsPart.js";

test("SessionsPart remains owned by the Sessions product layer", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const onDidChange = new Emitter<void>();
  const sessionService: IWorkbenchSessionService = {
    onDidChange: onDidChange.event,
    sessions: [],
    active: undefined,
    state: "ready",
    error: undefined,
    async initialize() {},
    selectThread() {},
    async ensureActiveThread() {
      throw new Error("No active thread");
    },
    async startNewSession() {
      throw new Error("Session creation is unavailable");
    },
    async archiveSession() {
      throw new Error("Session archiving is unavailable");
    },
  };
  const part = new SessionsPart(dom.window.document, sessionService);
  dom.window.document.body.append(part.element);

  assert.equal(part.element.dataset.part, "sessions");
  assert.equal(
    part.element.querySelector(".zeta-sessions-label")?.textContent,
    "No session",
  );

  part.dispose();
  onDidChange.dispose();
  dom.window.close();
});

import "./media/debugBreakpointDecorations.css";
import { Emitter, type Event } from "../../../../base/common/event.js";
import { Disposable } from "../../../../base/common/lifecycle.js";
import { type URI } from "../../../../base/common/uri.js";
import { type EditorLineGutterDecoration, type EditorLineGutterItem } from "../../../../editor/browser/viewparts/margin/lineGutterDecoration.js";
import { type IDebugService } from "../../../services/debug/common/debugService.js";

/** Supplies Debug breakpoint state to the editor's shared gutter renderer. */
export class DebugBreakpointDecorationProvider extends Disposable implements EditorLineGutterDecoration {
	private readonly changeEmitter = this._register(new Emitter<void>());
	readonly onDidChange: Event<void> = this.changeEmitter.event;

	constructor(private readonly debug: IDebugService, private readonly resource: URI) {
		super();
		this._register(debug.onDidChangeBreakpoints(() => this.changeEmitter.fire()));
	}

	getDecoration(logicalLineIndex: number, firstForLogicalLine: boolean): EditorLineGutterItem | undefined {
		if (!firstForLogicalLine) return undefined;
		const breakpoint = this.debug.breakpoints.find(candidate => candidate.resource.toString() === this.resource.toString() && candidate.lineNumber === logicalLineIndex + 1);
		const label = breakpoint ? `Remove breakpoint at line ${logicalLineIndex + 1}` : `Add breakpoint at line ${logicalLineIndex + 1}`;
		return {
			className: ["zeta-debug-breakpoint-gutter", breakpoint ? "checked" : "", breakpoint?.verified === true ? "verified" : ""].filter(Boolean).join(" "),
			label,
			title: label,
			pressed: Boolean(breakpoint),
		};
	}

	activate(logicalLineIndex: number): void {
		this.debug.toggleBreakpoint(this.resource, logicalLineIndex + 1);
	}
}

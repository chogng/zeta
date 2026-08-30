import { Disposable } from "../../../../base/common/lifecycle.js";
import { type CursorsController } from "../../../common/cursor/cursor.js";
import { createInsertFinalNewLineCommand } from "../common/finalNewLineEditCommand.js";

/** Applies the final-newline policy immediately before a save operation. */
export class InsertFinalNewLineController extends Disposable {
	constructor(private readonly selections: CursorsController) {
		super();
	}

	prepareSave(): void {
		const command = createInsertFinalNewLineCommand(this.selections.textModel, this.selections.selections);
		if (command) this.selections.execute(command);
	}
}

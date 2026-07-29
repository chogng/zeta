import {
  registerAction2,
} from "../../platform/actions/common/actions.js";
import {
  ToggleDeveloperToolsAction,
} from "./actions/developerActions.js";
import "./windowTheme.contribution.js";

registerAction2(ToggleDeveloperToolsAction);

import { bootstrapElectronMain } from "./bootstrap.js";

bootstrapElectronMain();
await import("./zeta/code/electron-main/codeMain.js");

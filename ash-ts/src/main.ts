import { bootstrapElectronMain } from "./bootstrap.js";

bootstrapElectronMain();
await import("./ash/code/electron-main/main.js");

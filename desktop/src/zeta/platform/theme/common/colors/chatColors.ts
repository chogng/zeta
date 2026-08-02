import { registerColor } from "../colorRegistry.js";

const owner = "chat.presentation";
const color = (id: string, dark: string, light: string, description: string): string => registerColor(id, { dark, light }, { description, owner });

export const chatTabBackground = color("chat.tabBackground", "#F8F8F8", "#F8F8F8", "Background for inactive Chat session tabs.");
export const chatTabActiveBackground = color("chat.tabActiveBackground", "#EEEEEE", "#EEEEEE", "Background for the active Chat session tab.");

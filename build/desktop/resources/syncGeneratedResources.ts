const { compileDesignTokens } = await import("../compileDesignTokens.ts");
await compileDesignTokens(true);
await import("./syncAppServerProtocol.ts");
const { checkIcons } = await import("../../resources/icons/generate.ts");
await checkIcons();

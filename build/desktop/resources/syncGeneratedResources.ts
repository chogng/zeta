const { compileDesignTokens } = await import("../compileDesignTokens.ts");
await compileDesignTokens(true);
await import("./syncAppServerProtocol.ts");
await import("./syncFileIcons.ts");
const { checkIcons } = await import("../../resources/icons/generate.ts");
await checkIcons();

const { compileDesignTokens } = await import("../compileDesignTokens.ts");
await compileDesignTokens(true);
await import("./syncAppServerProtocol.ts");
await import("./syncFileIcons.ts");
const { checkProductIcons, syncProductIcons } = await import("./syncProductIcons.ts");
await checkProductIcons();
await syncProductIcons({ writeSources: false });

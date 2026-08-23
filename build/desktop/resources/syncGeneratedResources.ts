const { compileDesignTokens } = await import("../compileDesignTokens.ts");
await compileDesignTokens(true);
await import("./syncAppServerProtocol.ts");
await import("./syncFileIcons.ts");
const { syncProductIcons } = await import("./syncProductIcons.ts");
await syncProductIcons({ sourceHandling: "check" });

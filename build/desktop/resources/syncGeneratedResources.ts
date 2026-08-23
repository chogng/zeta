const { compileDesignTokens } = await import("../compileDesignTokens.ts");
await compileDesignTokens(true);
await import("./syncAppServerProtocol.ts");
await import("./syncFileIcons.ts");
const { syncDesktopLicenseCopies } = await import("./syncDesktopLicenses.ts");
await syncDesktopLicenseCopies();
const { checkProductIcons, syncProductIcons } = await import("./syncProductIcons.ts");
await checkProductIcons();
await syncProductIcons({ writeSources: false });

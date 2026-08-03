const { compileDesignTokens } = await import("./compile-design-tokens.mjs");
await compileDesignTokens(true);
await import("./sync-app-server-protocol.mjs");
await import("./sync-file-icons.mjs");
const { checkProductIcons, syncProductIcons } = await import("./sync-product-icons.mjs");
await checkProductIcons();
await syncProductIcons({ writeSources: false });

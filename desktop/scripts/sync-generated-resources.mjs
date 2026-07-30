await import("./compile-design-tokens.mjs");
await import("./sync-app-server-protocol.mjs");
await import("./sync-file-icons.mjs");
const { syncProductIcons } = await import("./sync-product-icons.mjs");
await syncProductIcons();

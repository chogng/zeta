export type StartupWindowPageState =
  | {
    readonly kind: "starting";
    readonly message: string;
  }
  | {
    readonly kind: "failed";
    readonly message: string;
  };

function escapeHtml(value: string): string {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

/**
 * Creates the inert document shown while Electron validates the App Server.
 */
export function createStartupWindowDocument(
  productName: string,
  state: StartupWindowPageState,
): string {
  const failed = state.kind === "failed";
  const indicator = failed
    ? '<div class="failure-mark" aria-hidden="true">!</div>'
    : '<div class="spinner" aria-hidden="true"></div>';
  const status = failed ? "Startup could not continue" : "Starting";

  return `<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta
      http-equiv="Content-Security-Policy"
      content="default-src 'none'; style-src 'unsafe-inline';"
    >
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>${escapeHtml(productName)}</title>
    <style>
      :root {
        color-scheme: dark;
        font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
        background: #181818;
        color: #cccccc;
      }
      * { box-sizing: border-box; }
      body {
        min-height: 100vh;
        margin: 0;
        display: grid;
        place-items: center;
        background:
          radial-gradient(circle at 50% 20%, #252525 0, #181818 58%);
      }
      main {
        width: min(420px, calc(100vw - 48px));
        text-align: center;
      }
      .spinner,
      .failure-mark {
        width: 38px;
        height: 38px;
        margin: 0 auto 24px;
      }
      .spinner {
        border: 2px solid #3a3a3a;
        border-top-color: #8ab4f8;
        border-radius: 50%;
        animation: spin 900ms linear infinite;
      }
      .failure-mark {
        display: grid;
        place-items: center;
        border: 1px solid #f48771;
        border-radius: 50%;
        color: #f48771;
        font-size: 22px;
        font-weight: 600;
      }
      h1 {
        margin: 0 0 10px;
        color: #f0f0f0;
        font-size: 22px;
        font-weight: 500;
      }
      .status {
        margin: 0 0 8px;
        color: #9d9d9d;
        font-size: 12px;
        letter-spacing: 0.08em;
        text-transform: uppercase;
      }
      .message {
        min-height: 40px;
        margin: 0;
        color: #b8b8b8;
        font-size: 14px;
        line-height: 1.5;
      }
      @keyframes spin { to { transform: rotate(360deg); } }
      @media (prefers-reduced-motion: reduce) {
        .spinner { animation: none; border-top-color: #8ab4f8; }
      }
    </style>
  </head>
  <body>
    <main>
      ${indicator}
      <p class="status">${status}</p>
      <h1>${escapeHtml(productName)}</h1>
      <p class="message">${escapeHtml(state.message)}</p>
    </main>
  </body>
</html>`;
}

/** Converts the inert startup document into an opaque data URL. */
export function createStartupWindowUrl(
  productName: string,
  state: StartupWindowPageState,
): string {
  return `data:text/html;charset=utf-8,${
    encodeURIComponent(createStartupWindowDocument(productName, state))
  }`;
}

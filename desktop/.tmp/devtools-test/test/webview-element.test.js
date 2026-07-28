import assert from "node:assert/strict";
import test from "node:test";
class FakeContentWindow {
    messages = [];
    postMessage(message, targetOrigin, transfer) {
        this.messages.push({ message, targetOrigin, transfer });
    }
}
class FakeIframe extends EventTarget {
    contentWindow = new FakeContentWindow();
    style = {};
    attributes = new Map();
    name = "";
    className = "";
    tabIndex = -1;
    srcdoc = "";
    focused = false;
    removed = false;
    setAttribute(name, value) {
        this.attributes.set(name, value);
    }
    getAttribute(name) {
        return this.attributes.get(name) ?? null;
    }
    focus() {
        this.focused = true;
    }
    remove() {
        this.removed = true;
    }
}
class FakeWindow extends EventTarget {
}
class FakeDocument {
    defaultView = new FakeWindow();
    iframe = new FakeIframe();
    createElement(tagName) {
        assert.equal(tagName, "iframe");
        return this.iframe;
    }
}
function dispatchMessage(target, source, data) {
    const event = new Event("message");
    Object.defineProperties(event, {
        source: { value: source },
        data: { value: data },
    });
    target.dispatchEvent(event);
}
const { WebviewElement, } = await import("../src/platform/webview/browser/webviewElement.js");
test("webview element creates an opaque-origin sandbox document", () => {
    const ownerDocument = new FakeDocument();
    const webview = new WebviewElement({
        ownerDocument: ownerDocument,
        title: "Markdown preview",
        initialHtml: "<h1>Preview</h1>",
    });
    assert.equal(ownerDocument.iframe.getAttribute("sandbox"), "allow-scripts");
    assert.equal(ownerDocument.iframe.getAttribute("sandbox")?.includes("allow-same-origin"), false);
    assert.equal(ownerDocument.iframe.getAttribute("referrerpolicy"), "no-referrer");
    assert.equal(ownerDocument.iframe.getAttribute("credentialless"), "");
    assert.match(ownerDocument.iframe.getAttribute("csp") ?? "", /connect-src 'none'/);
    assert.match(ownerDocument.iframe.srcdoc, /Content-Security-Policy/);
    assert.match(ownerDocument.iframe.srcdoc, /default-src 'none'/);
    assert.match(ownerDocument.iframe.srcdoc, /acquireZetaWebviewApi/);
    assert.match(ownerDocument.iframe.srcdoc, /<h1>Preview<\/h1>/);
    webview.dispose();
    assert.equal(ownerDocument.iframe.removed, true);
    assert.equal(ownerDocument.iframe.srcdoc, "");
});
test("webview messages require the owned iframe source and channel", () => {
    const ownerDocument = new FakeDocument();
    const webview = new WebviewElement({
        ownerDocument: ownerDocument,
    });
    const messages = [];
    const registration = webview.onDidMessage((message) => messages.push(message));
    const channel = ownerDocument.iframe.getAttribute("data-zeta-webview-channel");
    dispatchMessage(ownerDocument.defaultView, {}, { channel, message: "wrong source" });
    dispatchMessage(ownerDocument.defaultView, ownerDocument.iframe.contentWindow, { channel: "wrong-channel", message: "wrong channel" });
    dispatchMessage(ownerDocument.defaultView, ownerDocument.iframe.contentWindow, { channel, message: { type: "ready" } });
    assert.deepEqual(messages, [{ type: "ready" }]);
    registration.dispose();
    webview.dispose();
});
test("webview host messaging and content replacement stop after disposal", () => {
    const ownerDocument = new FakeDocument();
    const webview = new WebviewElement({
        ownerDocument: ownerDocument,
    });
    assert.equal(webview.postMessage({ type: "update" }), true);
    assert.deepEqual(ownerDocument.iframe.contentWindow.messages, [{
            message: { type: "update" },
            targetOrigin: "*",
            transfer: [],
        }]);
    webview.setHtml("<p>Next</p>");
    assert.match(ownerDocument.iframe.srcdoc, /<p>Next<\/p>/);
    webview.focus();
    assert.equal(ownerDocument.iframe.focused, true);
    webview.dispose();
    assert.equal(webview.postMessage("late"), false);
    assert.throws(() => webview.setHtml("late"), /already disposed/);
});

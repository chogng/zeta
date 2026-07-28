import { BROWSER_VIEW_CLOSE_CHANNEL, BROWSER_VIEW_CREATE_CHANNEL, BROWSER_VIEW_GO_BACK_CHANNEL, BROWSER_VIEW_GO_FORWARD_CHANNEL, BROWSER_VIEW_LAYOUT_CHANNEL, BROWSER_VIEW_NAVIGATE_CHANNEL, BROWSER_VIEW_RELOAD_CHANNEL, BROWSER_VIEW_STATE_CHANNEL, BROWSER_VIEW_STOP_CHANNEL, BROWSER_VIEW_VISIBILITY_CHANNEL, validateBrowserViewCreateRequest, validateBrowserViewLayoutRequest, validateBrowserViewNavigateRequest, validateBrowserViewTargetRequest, validateBrowserViewVisibilityRequest, } from "../common/browserView.js";
/** Binds the main-owned browser-view service to trusted workbench IPC. */
export function browserViewIpcRoutes(service) {
    return [
        {
            channel: BROWSER_VIEW_CREATE_CHANNEL,
            validate: validateBrowserViewCreateRequest,
            invoke: (request) => service.createTarget(request),
        },
        {
            channel: BROWSER_VIEW_STATE_CHANNEL,
            validate: validateBrowserViewTargetRequest,
            invoke: (request) => service.observe(request.targetId),
        },
        {
            channel: BROWSER_VIEW_LAYOUT_CHANNEL,
            validate: validateBrowserViewLayoutRequest,
            invoke: (request) => service.layout(request),
        },
        {
            channel: BROWSER_VIEW_VISIBILITY_CHANNEL,
            validate: validateBrowserViewVisibilityRequest,
            invoke: (request) => service.setVisibility(request),
        },
        {
            channel: BROWSER_VIEW_NAVIGATE_CHANNEL,
            validate: validateBrowserViewNavigateRequest,
            invoke: (request) => service.navigate(request),
        },
        targetRoute(BROWSER_VIEW_GO_BACK_CHANNEL, (targetId) => service.goBack(targetId)),
        targetRoute(BROWSER_VIEW_GO_FORWARD_CHANNEL, (targetId) => service.goForward(targetId)),
        targetRoute(BROWSER_VIEW_RELOAD_CHANNEL, (targetId) => service.reload(targetId)),
        targetRoute(BROWSER_VIEW_STOP_CHANNEL, (targetId) => service.stop(targetId)),
        targetRoute(BROWSER_VIEW_CLOSE_CHANNEL, (targetId) => service.close(targetId)),
    ];
}
function targetRoute(channel, invoke) {
    return {
        channel,
        validate: validateBrowserViewTargetRequest,
        invoke: (request) => invoke(request.targetId),
    };
}

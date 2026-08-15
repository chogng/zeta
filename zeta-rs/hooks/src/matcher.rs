use crate::protocol::HookInvocation;
use zeta_config::HookConfig;

pub(crate) fn matches_event(hook: &HookConfig, invocation: &HookInvocation<'_>) -> bool {
    if hook.event != invocation.config_event() {
        return false;
    }
    match invocation.tool_name() {
        Some(tool_name) => {
            hook.matcher.tool_names.is_empty() || hook.matcher.tool_names.contains(tool_name)
        }
        None => hook.matcher.tool_names.is_empty(),
    }
}

#[cfg(test)]
#[path = "matcher_tests.rs"]
mod tests;

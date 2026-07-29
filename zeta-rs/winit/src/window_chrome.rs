use winit::window::WindowAttributes;

/// Selects how native window chrome and product-drawn content share the top window region.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum WindowChrome {
    /// Keeps the platform's standard titlebar and content layout.
    #[default]
    Native,
    /// Extends product content beneath a transparent macOS titlebar while retaining system window
    /// buttons and window-management behavior.
    ContentUnderTitlebar,
}

/// Applies a named chrome policy without exposing platform extension traits to product hosts.
pub fn apply_window_chrome(attributes: WindowAttributes, chrome: WindowChrome) -> WindowAttributes {
    match chrome {
        WindowChrome::Native => attributes,
        WindowChrome::ContentUnderTitlebar => content_under_titlebar(attributes),
    }
}

#[cfg(target_os = "macos")]
fn content_under_titlebar(attributes: WindowAttributes) -> WindowAttributes {
    use winit::platform::macos::WindowAttributesExtMacOS;

    attributes
        .with_titlebar_transparent(true)
        .with_title_hidden(true)
        .with_fullsize_content_view(true)
        .with_accepts_first_mouse(true)
}

#[cfg(not(target_os = "macos"))]
fn content_under_titlebar(attributes: WindowAttributes) -> WindowAttributes {
    attributes
}

#[cfg(test)]
#[path = "window_chrome_tests.rs"]
mod tests;

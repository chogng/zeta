use super::{WindowChrome, apply_window_chrome};
use winit::window::WindowAttributes;

#[test]
fn chrome_policy_preserves_product_window_attributes() {
    let attributes = WindowAttributes::default()
        .with_title("Product title")
        .with_resizable(false);

    for chrome in [WindowChrome::Native, WindowChrome::ContentUnderTitlebar] {
        let configured = apply_window_chrome(attributes.clone(), chrome);
        assert_eq!(configured.title, "Product title");
        assert!(!configured.resizable);
    }
}

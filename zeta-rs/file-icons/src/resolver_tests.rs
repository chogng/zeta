use super::*;
use crate::bundled_seti_manifest;

#[test]
fn resolver_applies_name_compound_extension_and_language_precedence() {
    let manifest = bundled_seti_manifest();
    let resolve = |file_name, scheme| {
        resolve_file_icon(manifest, file_name, scheme)
            .expect("bundled manifest has a default icon")
            .icon_id
    };

    assert_eq!(resolve("README.md", SetiColorScheme::Dark), "_info");
    assert_eq!(
        resolve("main.test.ts", SetiColorScheme::Dark),
        "_typescript_1"
    );
    assert_eq!(resolve("main.ts", SetiColorScheme::Dark), "_typescript");
    assert_eq!(resolve("main.RS", SetiColorScheme::Dark), "_rust");
    assert_eq!(resolve("source.unknown", SetiColorScheme::Dark), "_default");
    assert_eq!(
        resolve("main.ts", SetiColorScheme::Light),
        "_typescript_light"
    );
}

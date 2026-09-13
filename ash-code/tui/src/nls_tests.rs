use super::Language;
use super::Message;
use super::text;

#[test]
fn languages_cycle_in_both_directions() {
    let languages = [
        Language::English,
        Language::Japanese,
        Language::Chinese,
        Language::French,
    ];

    assert_eq!(
        languages.map(Language::next),
        [
            Language::Japanese,
            Language::Chinese,
            Language::French,
            Language::English,
        ]
    );
    assert_eq!(
        languages.map(Language::previous),
        [
            Language::French,
            Language::English,
            Language::Japanese,
            Language::Chinese,
        ]
    );
}

#[test]
fn nls_exposes_language_autonyms_and_typed_config_messages() {
    assert_eq!(Language::English.label(), "English");
    assert_eq!(Language::Japanese.label(), "日本語");
    assert_eq!(Language::Chinese.label(), "中文");
    assert_eq!(Language::French.label(), "Français");
    assert_eq!(
        text(Language::Chinese, Message::ConfigLanguageDescription),
        "切换界面语言"
    );
}

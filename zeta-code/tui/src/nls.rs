//! TUI display languages and localized messages.

use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum Language {
    #[serde(rename = "en")]
    English,
    #[serde(rename = "ja")]
    Japanese,
    #[serde(rename = "zh-CN")]
    Chinese,
    #[serde(rename = "fr")]
    French,
}

impl Language {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Japanese => "日本語",
            Self::Chinese => "中文",
            Self::French => "Français",
        }
    }

    pub(crate) const fn next(self) -> Self {
        match self {
            Self::English => Self::Japanese,
            Self::Japanese => Self::Chinese,
            Self::Chinese => Self::French,
            Self::French => Self::English,
        }
    }

    pub(crate) const fn previous(self) -> Self {
        match self {
            Self::English => Self::French,
            Self::Japanese => Self::English,
            Self::Chinese => Self::Japanese,
            Self::French => Self::Chinese,
        }
    }
}

pub(crate) const fn text(language: Language, message: Message) -> &'static str {
    match language {
        Language::English => english(message),
        Language::Japanese => japanese(message),
        Language::Chinese => chinese(message),
        Language::French => french(message),
    }
}

impl Default for Language {
    fn default() -> Self {
        Self::English
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Message {
    ConfigTitle,
    ConfigProviders,
    ConfigLanguageServers,
    ConfigEnhancedTui,
    ConfigEnhancedTuiDescription,
    ConfigVimMode,
    ConfigVimModeDescription,
    ConfigMemoryDiagnostics,
    ConfigMemoryDiagnosticsDescription,
    ConfigGitChangesAsDiff,
    ConfigGitChangesAsDiffDescription,
    ConfigLanguage,
    ConfigLanguageDescription,
    ConfigSearch,
    ConfigNoMatches,
    ConfigNoLanguageServers,
}

const fn english(message: Message) -> &'static str {
    match message {
        Message::ConfigTitle => "Config",
        Message::ConfigProviders => "Providers",
        Message::ConfigLanguageServers => "Language servers",
        Message::ConfigEnhancedTui => "Enhanced TUI",
        Message::ConfigEnhancedTuiDescription => {
            "Click, hover, drag-select text, and copy automatically"
        }
        Message::ConfigVimMode => "Vim mode",
        Message::ConfigVimModeDescription => "Use Vim editing in ChatInput",
        Message::ConfigMemoryDiagnostics => "Memory diagnostics",
        Message::ConfigMemoryDiagnosticsDescription => {
            "Continuously collect bounded memory evidence"
        }
        Message::ConfigGitChangesAsDiff => "Show Git changes as diff",
        Message::ConfigGitChangesAsDiffDescription => {
            "Show added and deleted lines instead of changed files"
        }
        Message::ConfigLanguage => "Language",
        Message::ConfigLanguageDescription => "Change the interface language",
        Message::ConfigSearch => "Search configuration",
        Message::ConfigNoMatches => "No matching configuration",
        Message::ConfigNoLanguageServers => "No language servers configured",
    }
}

const fn japanese(message: Message) -> &'static str {
    match message {
        Message::ConfigTitle => "設定",
        Message::ConfigProviders => "プロバイダー",
        Message::ConfigLanguageServers => "言語サーバー",
        Message::ConfigEnhancedTui => "拡張 TUI",
        Message::ConfigEnhancedTuiDescription => {
            "クリック、ホバー表示、文字のドラッグ選択、自動コピーを有効にする"
        }
        Message::ConfigVimMode => "Vim モード",
        Message::ConfigVimModeDescription => "ChatInput で Vim 編集を使用する",
        Message::ConfigMemoryDiagnostics => "メモリ診断",
        Message::ConfigMemoryDiagnosticsDescription => {
            "上限付きのメモリ診断データを継続的に収集する"
        }
        Message::ConfigGitChangesAsDiff => "Git の変更を差分で表示",
        Message::ConfigGitChangesAsDiffDescription => {
            "変更されたファイルではなく、追加・削除された行を表示する"
        }
        Message::ConfigLanguage => "言語",
        Message::ConfigLanguageDescription => "インターフェースの言語を変更する",
        Message::ConfigSearch => "設定を検索",
        Message::ConfigNoMatches => "一致する設定がありません",
        Message::ConfigNoLanguageServers => "設定された言語サーバーはありません",
    }
}

const fn chinese(message: Message) -> &'static str {
    match message {
        Message::ConfigTitle => "配置",
        Message::ConfigProviders => "提供商",
        Message::ConfigLanguageServers => "语言服务器",
        Message::ConfigEnhancedTui => "增强 TUI",
        Message::ConfigEnhancedTuiDescription => "启用点击、悬停反馈、拖选文字和自动复制",
        Message::ConfigVimMode => "Vim 模式",
        Message::ConfigVimModeDescription => "在 ChatInput 中使用 Vim 编辑",
        Message::ConfigMemoryDiagnostics => "内存诊断",
        Message::ConfigMemoryDiagnosticsDescription => "持续收集有界的内存诊断数据",
        Message::ConfigGitChangesAsDiff => "以差异显示 Git 更改",
        Message::ConfigGitChangesAsDiffDescription => "显示新增和删除的行，而不是已更改的文件",
        Message::ConfigLanguage => "语言",
        Message::ConfigLanguageDescription => "切换界面语言",
        Message::ConfigSearch => "搜索配置",
        Message::ConfigNoMatches => "没有匹配的配置",
        Message::ConfigNoLanguageServers => "未配置语言服务器",
    }
}

const fn french(message: Message) -> &'static str {
    match message {
        Message::ConfigTitle => "Configuration",
        Message::ConfigProviders => "Fournisseurs",
        Message::ConfigLanguageServers => "Serveurs de langage",
        Message::ConfigEnhancedTui => "TUI améliorée",
        Message::ConfigEnhancedTuiDescription => {
            "Activer les clics, le survol, la sélection par glissement et la copie automatique"
        }
        Message::ConfigVimMode => "Mode Vim",
        Message::ConfigVimModeDescription => "Utiliser l’édition Vim dans ChatInput",
        Message::ConfigMemoryDiagnostics => "Diagnostic mémoire",
        Message::ConfigMemoryDiagnosticsDescription => {
            "Collecter en continu des données de diagnostic mémoire limitées"
        }
        Message::ConfigGitChangesAsDiff => "Afficher les modifications Git sous forme de diff",
        Message::ConfigGitChangesAsDiffDescription => {
            "Afficher les lignes ajoutées et supprimées au lieu des fichiers modifiés"
        }
        Message::ConfigLanguage => "Langue",
        Message::ConfigLanguageDescription => "Changer la langue de l’interface",
        Message::ConfigSearch => "Rechercher dans la configuration",
        Message::ConfigNoMatches => "Aucune configuration correspondante",
        Message::ConfigNoLanguageServers => "Aucun serveur de langage configuré",
    }
}

#[cfg(test)]
#[path = "nls_tests.rs"]
mod tests;

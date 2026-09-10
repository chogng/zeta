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
    ConfigIssues,

    ConfigTitle,
    ConfigScreenMode,
    ConfigScreenModeDescription,
    ConfigGeneral,
    ConfigProviders,
    ConfigLanguageServers,
    ConfigVimMode,
    ConfigVimModeDescription,
    ConfigMemoryDiagnostics,
    ConfigMemoryDiagnosticsDescription,
    ConfigAutoUpdate,
    ConfigAutoUpdateDescription,
    ConfigUpdateLatest,
    ConfigUpdateStable,
    ConfigUpdateNever,
    ConfigGitChangesAsDiff,
    ConfigGitChangesAsDiffDescription,
    ConfigStatusLineStyle,
    ConfigStatusLineSimple,
    ConfigStatusLineExpressive,
    ConfigStatusLineSimpleDescription,
    ConfigStatusLineExpressiveDescription,
    ConfigLanguage,
    ConfigLanguageDescription,
    ConfigSearch,
    ConfigNoMatches,
    ConfigNoLanguageServers,
}

const fn english(message: Message) -> &'static str {
    match message {
        Message::ConfigIssues => "Issues",

        Message::ConfigTitle => "Config",
        Message::ConfigScreenMode => "Screen mode",
        Message::ConfigScreenModeDescription => "Use a full screen or keep terminal history",
        Message::ConfigGeneral => "General",
        Message::ConfigProviders => "Providers",
        Message::ConfigLanguageServers => "Language servers",
        Message::ConfigVimMode => "Vim mode",
        Message::ConfigVimModeDescription => "Use Vim editing in ChatInput",
        Message::ConfigMemoryDiagnostics => "Memory diagnostics",
        Message::ConfigMemoryDiagnosticsDescription => {
            "Continuously collect bounded memory evidence"
        }
        Message::ConfigAutoUpdate => "Automatic updates",
        Message::ConfigAutoUpdateDescription => "Choose release cadence",
        Message::ConfigUpdateLatest => "Latest",
        Message::ConfigUpdateStable => "Stable",
        Message::ConfigUpdateNever => "Never",
        Message::ConfigGitChangesAsDiff => "Show Git changes as diff",
        Message::ConfigGitChangesAsDiffDescription => {
            "Show added and deleted lines instead of changed files"
        }
        Message::ConfigStatusLineStyle => "Status bar style",
        Message::ConfigStatusLineSimple => "Simple",
        Message::ConfigStatusLineExpressive => "Expressive",
        Message::ConfigStatusLineSimpleDescription => "Text and numbers, clean and easy to read",
        Message::ConfigStatusLineExpressiveDescription => "Emoji and progress bars at a glance",
        Message::ConfigLanguage => "Language",
        Message::ConfigLanguageDescription => "Change the interface language",
        Message::ConfigSearch => "Search configuration",
        Message::ConfigNoMatches => "No matching configuration",
        Message::ConfigNoLanguageServers => "No language servers configured",
    }
}

const fn japanese(message: Message) -> &'static str {
    match message {
        Message::ConfigIssues => "Issues",

        Message::ConfigTitle => "設定",
        Message::ConfigScreenMode => "画面モード",
        Message::ConfigScreenModeDescription => "全画面表示またはターミナル履歴を保持",
        Message::ConfigGeneral => "一般",
        Message::ConfigProviders => "プロバイダー",
        Message::ConfigLanguageServers => "言語サーバー",
        Message::ConfigVimMode => "Vim モード",
        Message::ConfigVimModeDescription => "ChatInput で Vim 編集を使用する",
        Message::ConfigMemoryDiagnostics => "メモリ診断",
        Message::ConfigMemoryDiagnosticsDescription => {
            "上限付きのメモリ診断データを継続的に収集する"
        }
        Message::ConfigAutoUpdate => "自動更新",
        Message::ConfigAutoUpdateDescription => "リリース頻度を選択する",
        Message::ConfigUpdateLatest => "最新",
        Message::ConfigUpdateStable => "安定版",
        Message::ConfigUpdateNever => "なし",
        Message::ConfigGitChangesAsDiff => "Git の変更を差分で表示",
        Message::ConfigGitChangesAsDiffDescription => {
            "変更されたファイルではなく、追加・削除された行を表示する"
        }
        Message::ConfigStatusLineStyle => "ステータスバーの表示",
        Message::ConfigStatusLineSimple => "シンプル",
        Message::ConfigStatusLineExpressive => "華やか",
        Message::ConfigStatusLineSimpleDescription => "文字と数値ですっきり表示",
        Message::ConfigStatusLineExpressiveDescription => "絵文字と進捗バーでひと目で確認",
        Message::ConfigLanguage => "言語",
        Message::ConfigLanguageDescription => "インターフェースの言語を変更する",
        Message::ConfigSearch => "設定を検索",
        Message::ConfigNoMatches => "一致する設定がありません",
        Message::ConfigNoLanguageServers => "設定された言語サーバーはありません",
    }
}

const fn chinese(message: Message) -> &'static str {
    match message {
        Message::ConfigIssues => "Issues",

        Message::ConfigTitle => "配置",
        Message::ConfigScreenMode => "屏幕模式",
        Message::ConfigScreenModeDescription => "使用全屏界面或保留终端历史",
        Message::ConfigGeneral => "通用",
        Message::ConfigProviders => "提供商",
        Message::ConfigLanguageServers => "语言服务器",
        Message::ConfigVimMode => "Vim 模式",
        Message::ConfigVimModeDescription => "在 ChatInput 中使用 Vim 编辑",
        Message::ConfigMemoryDiagnostics => "内存诊断",
        Message::ConfigMemoryDiagnosticsDescription => "持续收集有界的内存诊断数据",
        Message::ConfigAutoUpdate => "自动更新",
        Message::ConfigAutoUpdateDescription => "选择版本更新节奏",
        Message::ConfigUpdateLatest => "最新",
        Message::ConfigUpdateStable => "稳定",
        Message::ConfigUpdateNever => "从不",
        Message::ConfigGitChangesAsDiff => "以差异显示 Git 更改",
        Message::ConfigGitChangesAsDiffDescription => "显示新增和删除的行，而不是已更改的文件",
        Message::ConfigStatusLineStyle => "状态栏风格",
        Message::ConfigStatusLineSimple => "简洁",
        Message::ConfigStatusLineExpressive => "生动",
        Message::ConfigStatusLineSimpleDescription => "文字与数值，清爽易读",
        Message::ConfigStatusLineExpressiveDescription => "加入表情与进度条，一眼看清状态",
        Message::ConfigLanguage => "语言",
        Message::ConfigLanguageDescription => "切换界面语言",
        Message::ConfigSearch => "搜索配置",
        Message::ConfigNoMatches => "没有匹配的配置",
        Message::ConfigNoLanguageServers => "未配置语言服务器",
    }
}

const fn french(message: Message) -> &'static str {
    match message {
        Message::ConfigIssues => "Issues",

        Message::ConfigTitle => "Configuration",
        Message::ConfigScreenMode => "Mode d’écran",
        Message::ConfigScreenModeDescription => "Plein écran ou historique du terminal",
        Message::ConfigGeneral => "Général",
        Message::ConfigProviders => "Fournisseurs",
        Message::ConfigLanguageServers => "Serveurs de langage",
        Message::ConfigVimMode => "Mode Vim",
        Message::ConfigVimModeDescription => "Utiliser l’édition Vim dans ChatInput",
        Message::ConfigMemoryDiagnostics => "Diagnostic mémoire",
        Message::ConfigMemoryDiagnosticsDescription => {
            "Collecter en continu des données de diagnostic mémoire limitées"
        }
        Message::ConfigAutoUpdate => "Mises à jour automatiques",
        Message::ConfigAutoUpdateDescription => "Choisir le rythme des versions",
        Message::ConfigUpdateLatest => "Dernière",
        Message::ConfigUpdateStable => "Stable",
        Message::ConfigUpdateNever => "Jamais",
        Message::ConfigGitChangesAsDiff => "Afficher les modifications Git sous forme de diff",
        Message::ConfigGitChangesAsDiffDescription => {
            "Afficher les lignes ajoutées et supprimées au lieu des fichiers modifiés"
        }
        Message::ConfigStatusLineStyle => "Style de la barre d’état",
        Message::ConfigStatusLineSimple => "Simple",
        Message::ConfigStatusLineExpressive => "Expressif",
        Message::ConfigStatusLineSimpleDescription => "Du texte et des chiffres, faciles à lire",
        Message::ConfigStatusLineExpressiveDescription => "Des emoji et des barres de progression",
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

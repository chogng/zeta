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
    ConfigIssueMerge,
    ConfigIssueMergeDescription,
    ConfigIssueModel,
    ConfigIssueModelDescription,
    ConfigIssueModelMissing,
    ConfigIssueModelPicker,
    ConfigIssueModelSearch,
    ConfigIssueModelEmpty,
    ConfigIssueModelClear,

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
        Message::ConfigIssues => "Issues",
        Message::ConfigIssueMerge => "Recommend issue grouping",
        Message::ConfigIssueMergeDescription => "Suggest issues to implement together",
        Message::ConfigIssueModel => "Analysis model",
        Message::ConfigIssueModelDescription => "Use this model to analyse related issues",
        Message::ConfigIssueModelMissing => "Not configured - choose a model",
        Message::ConfigIssueModelPicker => "Issue analysis model",
        Message::ConfigIssueModelSearch => "Search models",
        Message::ConfigIssueModelEmpty => "No models from configured providers",
        Message::ConfigIssueModelClear => "Clear model selection",

        Message::ConfigTitle => "Config",
        Message::ConfigProviders => "Providers",
        Message::ConfigLanguageServers => "Language servers",
        Message::ConfigEnhancedTui => "Enhanced TUI",
        Message::ConfigEnhancedTuiDescription => {
            "Click, scroll, hover, and auto-copy text in overlays only"
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
        Message::ConfigIssues => "Issues",
        Message::ConfigIssueMerge => "Issue のまとめ処理を提案",
        Message::ConfigIssueMergeDescription => "一緒に実装できる Issue を提案",
        Message::ConfigIssueModel => "分析モデル",
        Message::ConfigIssueModelDescription => "関連する Issue の分析に使うモデル",
        Message::ConfigIssueModelMissing => "未設定 - モデルを選択",
        Message::ConfigIssueModelPicker => "Issue 分析モデル",
        Message::ConfigIssueModelSearch => "モデルを検索",
        Message::ConfigIssueModelEmpty => "設定済みプロバイダーのモデルがありません",
        Message::ConfigIssueModelClear => "モデルの選択を解除",

        Message::ConfigTitle => "設定",
        Message::ConfigProviders => "プロバイダー",
        Message::ConfigLanguageServers => "言語サーバー",
        Message::ConfigEnhancedTui => "拡張 TUI",
        Message::ConfigEnhancedTuiDescription => {
            "オーバーレイ内のみでクリック、スクロール、ホバー、テキストの自動コピーを有効にする"
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
        Message::ConfigIssues => "Issues",
        Message::ConfigIssueMerge => "推荐合并处理",
        Message::ConfigIssueMergeDescription => "推荐适合一起解决的 issue",
        Message::ConfigIssueModel => "分析模型",
        Message::ConfigIssueModelDescription => "用于分析相似或重复的 issue",
        Message::ConfigIssueModelMissing => "尚未配置，请选择模型",
        Message::ConfigIssueModelPicker => "Issue 分析模型",
        Message::ConfigIssueModelSearch => "搜索模型",
        Message::ConfigIssueModelEmpty => "已配置供应商中没有可用模型",
        Message::ConfigIssueModelClear => "清除模型选择",

        Message::ConfigTitle => "配置",
        Message::ConfigProviders => "提供商",
        Message::ConfigLanguageServers => "语言服务器",
        Message::ConfigEnhancedTui => "增强 TUI",
        Message::ConfigEnhancedTuiDescription => "仅在浮层中启用点击、滚动、悬停和自动复制文本",
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
        Message::ConfigIssues => "Issues",
        Message::ConfigIssueMerge => "Suggérer des groupes d’issues",
        Message::ConfigIssueMergeDescription => "Suggérer les issues à traiter ensemble",
        Message::ConfigIssueModel => "Modèle d’analyse",
        Message::ConfigIssueModelDescription => "Analyser les issues liées avec ce modèle",
        Message::ConfigIssueModelMissing => "Non configuré - choisir un modèle",
        Message::ConfigIssueModelPicker => "Modèle d’analyse des issues",
        Message::ConfigIssueModelSearch => "Rechercher des modèles",
        Message::ConfigIssueModelEmpty => "Aucun modèle des fournisseurs configurés",
        Message::ConfigIssueModelClear => "Effacer le choix du modèle",

        Message::ConfigTitle => "Configuration",
        Message::ConfigProviders => "Fournisseurs",
        Message::ConfigLanguageServers => "Serveurs de langage",
        Message::ConfigEnhancedTui => "TUI améliorée",
        Message::ConfigEnhancedTuiDescription => {
            "Activer le clic, le défilement, le survol et la copie automatique dans les fenêtres superposées"
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

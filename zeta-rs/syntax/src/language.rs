use tree_sitter::{Language, Parser, Query};

use crate::SyntaxError;

/// Source language selected for syntax analysis.
///
/// Each variant identifies a grammar and the structural queries that implementations must use.
/// Adding a language requires registering its grammar and tests in this crate; callers do not
/// supply arbitrary query source or native grammar pointers.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SyntaxLanguage {
    Rust,
}

impl SyntaxLanguage {
    pub const fn id(self) -> &'static str {
        match self {
            Self::Rust => "rust",
        }
    }
}

pub(crate) struct LanguageConfiguration {
    pub(crate) highlights: Query,
    pub(crate) tags: Query,
}

impl LanguageConfiguration {
    pub(crate) fn load(
        syntax_language: SyntaxLanguage,
        parser: &mut Parser,
    ) -> Result<Self, SyntaxError> {
        let language = language(syntax_language);
        parser
            .set_language(&language)
            .map_err(|source| SyntaxError::Language {
                language: syntax_language,
                source,
            })?;
        let highlights = query(
            syntax_language,
            &language,
            "highlights",
            tree_sitter_rust::HIGHLIGHTS_QUERY,
        )?;
        let tags = query(
            syntax_language,
            &language,
            "tags",
            tree_sitter_rust::TAGS_QUERY,
        )?;
        Ok(Self { highlights, tags })
    }
}

fn language(syntax_language: SyntaxLanguage) -> Language {
    match syntax_language {
        SyntaxLanguage::Rust => tree_sitter_rust::LANGUAGE.into(),
    }
}

fn query(
    syntax_language: SyntaxLanguage,
    language: &Language,
    query_name: &'static str,
    source: &str,
) -> Result<Query, SyntaxError> {
    Query::new(language, source).map_err(|source| SyntaxError::Query {
        language: syntax_language,
        query_name,
        source,
    })
}

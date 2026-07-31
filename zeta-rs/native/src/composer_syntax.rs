use zeta_editor::{CodeEditorSyntaxHighlighter, CodeEditorSyntaxToken, CodeEditorTokenRole};
use zeta_syntax::{
    DocumentRevision, SyntaxDocument, SyntaxEdit, SyntaxLanguage, SyntaxSnapshot, SyntaxTokenKind,
};

pub(crate) struct ComposerShellSyntax {
    document: Option<SyntaxDocument>,
    revision: u64,
}

impl ComposerShellSyntax {
    pub(crate) const fn new() -> Self {
        Self {
            document: None,
            revision: 0,
        }
    }

    pub(crate) fn synchronize(&mut self, text: &str) -> Option<ComposerSyntaxProjection> {
        let next_revision = self.revision.saturating_add(1).max(1);
        let snapshot = match self.document.as_mut() {
            Some(document) if document.text() == text => document.snapshot(),
            Some(document) => {
                let edit = replacement_between(document.text(), text);
                match document.apply_edit(DocumentRevision::new(next_revision), &edit) {
                    Ok(snapshot) => {
                        self.revision = next_revision;
                        snapshot
                    }
                    Err(_) => return self.reopen(text, next_revision),
                }
            }
            None => return self.reopen(text, next_revision),
        };
        Some(ComposerSyntaxProjection::from_snapshot(text, &snapshot))
    }

    fn reopen(&mut self, text: &str, revision: u64) -> Option<ComposerSyntaxProjection> {
        let document =
            SyntaxDocument::open(SyntaxLanguage::Shell, DocumentRevision::new(revision), text)
                .ok()?;
        let projection =
            ComposerSyntaxProjection::from_snapshot(document.text(), &document.snapshot());
        self.document = Some(document);
        self.revision = revision;
        Some(projection)
    }
}

impl Default for ComposerShellSyntax {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default)]
pub(crate) struct ComposerSyntaxProjection {
    lines: Vec<Vec<CodeEditorSyntaxToken>>,
}

impl ComposerSyntaxProjection {
    fn from_snapshot(text: &str, snapshot: &SyntaxSnapshot) -> Self {
        let source_lines = text.split('\n').collect::<Vec<_>>();
        let mut lines = vec![Vec::new(); source_lines.len().max(1)];
        for token in snapshot.tokens() {
            let last_row = token.range.end.row.min(lines.len().saturating_sub(1));
            for row in token.range.start.row..=last_row {
                let line = source_lines.get(row).copied().unwrap_or_default();
                let start = if row == token.range.start.row {
                    token.range.start.column.min(line.len())
                } else {
                    0
                };
                let end = if row == token.range.end.row {
                    token.range.end.column.min(line.len())
                } else {
                    line.len()
                };
                if start < end && line.is_char_boundary(start) && line.is_char_boundary(end) {
                    lines[row].push(CodeEditorSyntaxToken::new(
                        start..end,
                        token_role(token.kind),
                    ));
                }
            }
        }
        Self { lines }
    }
}

impl CodeEditorSyntaxHighlighter for ComposerSyntaxProjection {
    fn highlight_line(&self, line_number: usize, _text: &str) -> Vec<CodeEditorSyntaxToken> {
        line_number
            .checked_sub(1)
            .and_then(|index| self.lines.get(index))
            .cloned()
            .unwrap_or_default()
    }
}

pub(crate) struct ComposerPlainTextSyntax;

impl CodeEditorSyntaxHighlighter for ComposerPlainTextSyntax {
    fn highlight_line(&self, _line_number: usize, _text: &str) -> Vec<CodeEditorSyntaxToken> {
        Vec::new()
    }
}

fn replacement_between(current: &str, next: &str) -> SyntaxEdit {
    let mut prefix = current
        .as_bytes()
        .iter()
        .zip(next.as_bytes())
        .take_while(|(left, right)| left == right)
        .count();
    while prefix > 0 && (!current.is_char_boundary(prefix) || !next.is_char_boundary(prefix)) {
        prefix -= 1;
    }
    let suffix = current[prefix..]
        .chars()
        .rev()
        .zip(next[prefix..].chars().rev())
        .take_while(|(left, right)| left == right)
        .map(|(character, _)| character.len_utf8())
        .sum::<usize>();
    SyntaxEdit::replace(
        prefix..current.len() - suffix,
        &next[prefix..next.len() - suffix],
    )
}

fn token_role(kind: SyntaxTokenKind) -> CodeEditorTokenRole {
    match kind {
        SyntaxTokenKind::Attribute => CodeEditorTokenRole::Attribute,
        SyntaxTokenKind::Comment => CodeEditorTokenRole::Comment,
        SyntaxTokenKind::Constant => CodeEditorTokenRole::Constant,
        SyntaxTokenKind::Constructor => CodeEditorTokenRole::Constructor,
        SyntaxTokenKind::Embedded => CodeEditorTokenRole::Embedded,
        SyntaxTokenKind::Function => CodeEditorTokenRole::Function,
        SyntaxTokenKind::Keyword => CodeEditorTokenRole::Keyword,
        SyntaxTokenKind::Label => CodeEditorTokenRole::Label,
        SyntaxTokenKind::Module => CodeEditorTokenRole::Module,
        SyntaxTokenKind::Number => CodeEditorTokenRole::Number,
        SyntaxTokenKind::Operator => CodeEditorTokenRole::Operator,
        SyntaxTokenKind::Property => CodeEditorTokenRole::Property,
        SyntaxTokenKind::Punctuation => CodeEditorTokenRole::Punctuation,
        SyntaxTokenKind::String => CodeEditorTokenRole::String,
        SyntaxTokenKind::Type => CodeEditorTokenRole::Type,
        SyntaxTokenKind::Variable => CodeEditorTokenRole::Variable,
    }
}

#[cfg(test)]
#[path = "composer_syntax_tests.rs"]
mod tests;

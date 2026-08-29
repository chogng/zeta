#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct KeyCapture {
    title: String,
    lines: Vec<String>,
}

impl KeyCapture {
    pub(crate) fn new(title: impl Into<String>, lines: Vec<String>) -> Self {
        Self {
            title: title.into(),
            lines,
        }
    }

    pub(crate) fn title(&self) -> &str {
        &self.title
    }

    pub(crate) fn lines(&self) -> &[String] {
        &self.lines
    }

    pub(crate) fn desired_height(&self) -> u16 {
        u16::try_from(self.lines.len().saturating_add(3)).unwrap_or(u16::MAX)
    }
}

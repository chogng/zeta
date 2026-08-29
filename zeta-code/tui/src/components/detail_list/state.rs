#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DetailListRow {
    label: String,
    value: String,
}

impl DetailListRow {
    pub(crate) fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
        }
    }

    pub(crate) fn label(&self) -> &str {
        &self.label
    }

    pub(crate) fn value(&self) -> &str {
        &self.value
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DetailList {
    title: String,
    rows: Vec<DetailListRow>,
}

impl DetailList {
    pub(crate) fn new(title: impl Into<String>, rows: Vec<DetailListRow>) -> Self {
        Self {
            title: title.into(),
            rows,
        }
    }

    pub(crate) fn title(&self) -> &str {
        &self.title
    }

    pub(crate) fn rows(&self) -> &[DetailListRow] {
        &self.rows
    }

    pub(crate) fn desired_height(&self) -> u16 {
        u16::try_from(self.rows.len().saturating_add(3)).unwrap_or(u16::MAX)
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;

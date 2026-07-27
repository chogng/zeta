use crate::ImageDetail;

/// The model-visible success classification returned by an executable tool.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ToolOutputStatus {
    Success,
    Error,
}

/// One bounded piece of model-visible tool output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ToolContent {
    Text(String),
    Image { url: String, detail: ImageDetail },
}

/// Provider-neutral output returned after a tool reaches a trustworthy terminal result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolOutput {
    status: ToolOutputStatus,
    content: Vec<ToolContent>,
}

impl ToolOutput {
    pub fn success(content: Vec<ToolContent>) -> Self {
        Self {
            status: ToolOutputStatus::Success,
            content,
        }
    }

    pub fn error(content: Vec<ToolContent>) -> Self {
        Self {
            status: ToolOutputStatus::Error,
            content,
        }
    }

    pub fn status(&self) -> ToolOutputStatus {
        self.status
    }

    pub fn content(&self) -> &[ToolContent] {
        &self.content
    }
}

#[cfg(test)]
#[path = "output_tests.rs"]
mod tests;

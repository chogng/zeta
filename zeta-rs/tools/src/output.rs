use crate::ImageDetail;
use crate::ImageDetailCapabilities;
use crate::ImageDetailDecision;
use crate::ImageDetailSelection;
use crate::ImageSourceDetailPolicy;
use crate::normalize_image_detail;

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

    /// Applies the final model-capability image gate to every image in this output.
    pub fn sanitize_image_detail(
        &mut self,
        capabilities: ImageDetailCapabilities,
        source_policy: ImageSourceDetailPolicy,
    ) -> Vec<ImageDetailDecision> {
        let mut decisions = Vec::new();
        for content in &mut self.content {
            let ToolContent::Image { detail, .. } = content else {
                continue;
            };
            let decision = normalize_image_detail(
                ImageDetailSelection::Explicit(*detail),
                capabilities,
                source_policy,
            );
            *detail = match decision.effective {
                ImageDetailSelection::ProviderDefault => ImageDetail::Auto,
                ImageDetailSelection::Explicit(detail) => detail,
            };
            decisions.push(decision);
        }
        decisions
    }
}

#[cfg(test)]
#[path = "output_tests.rs"]
mod tests;

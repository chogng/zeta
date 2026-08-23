use std::sync::Arc;

use zeta_async_utils::CancellationToken;
use zeta_attachments::ImageAttachments;
use zeta_protocol::ContentPart;
use zeta_protocol::InputItem;
use zeta_protocol::ModelRequest;
use zeta_protocol::ModelResponse;

use crate::ContextBudget;
use crate::ContextTokenMeasurementCapability;
use crate::ContextTokenMeasurementOutcome;
use crate::CoreError;
use crate::ModelSelection;
use crate::ModelService;
use crate::ModelStreamSink;

pub(crate) struct AttachmentModelService {
    inner: Arc<dyn ModelService>,
    attachments: Arc<ImageAttachments>,
}

impl AttachmentModelService {
    pub(crate) fn new(inner: Arc<dyn ModelService>, attachments: Arc<ImageAttachments>) -> Self {
        Self { inner, attachments }
    }

    fn materialize(
        &self,
        selection: ModelSelection<'_>,
        request: &ModelRequest,
    ) -> Result<ModelRequest, CoreError> {
        let policy = self.inner.image_input_policy(selection)?;
        let mut materialized = request.clone();
        for item in &mut materialized.input {
            let content = match item {
                InputItem::Message(message) => &mut message.content,
                InputItem::ToolResult(result) => &mut result.content,
            };
            for part in content {
                let replacement = match part {
                    ContentPart::ImageAttachment { attachment, detail } => {
                        let limits = policy.limits_for(*detail);
                        let data_url = self
                            .attachments
                            .materialize_data_url_with_limits(
                                attachment,
                                zeta_utils_image::PromptImageResizeLimits {
                                    max_dimension: limits.max_dimension,
                                    max_patches: limits.max_patches,
                                },
                            )
                            .map_err(|error| CoreError::Context(error.to_string()))?;
                        Some(ContentPart::ImageUrl {
                            url: data_url,
                            detail: *detail,
                        })
                    }
                    ContentPart::ImageUrl { url, detail } if is_data_url(url) => {
                        let limits = policy.limits_for(*detail);
                        let data_url = self
                            .attachments
                            .prepare_data_url_with_limits(
                                url,
                                zeta_utils_image::PromptImageResizeLimits {
                                    max_dimension: limits.max_dimension,
                                    max_patches: limits.max_patches,
                                },
                            )
                            .map_err(|error| CoreError::Context(error.to_string()))?;
                        Some(ContentPart::ImageUrl {
                            url: data_url,
                            detail: *detail,
                        })
                    }
                    ContentPart::Text(_) | ContentPart::ImageUrl { .. } => None,
                };
                if let Some(replacement) = replacement {
                    *part = replacement;
                }
            }
        }
        Ok(materialized)
    }
}

fn is_data_url(url: &str) -> bool {
    url.get(.."data:".len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("data:"))
}

impl ModelService for AttachmentModelService {
    fn context_budget(&self, selection: ModelSelection<'_>) -> Result<ContextBudget, CoreError> {
        self.inner.context_budget(selection)
    }

    fn image_input_policy(
        &self,
        selection: ModelSelection<'_>,
    ) -> Result<crate::ModelImageInputPolicy, CoreError> {
        self.inner.image_input_policy(selection)
    }

    fn input_token_measurement_capability(
        &self,
        selection: ModelSelection<'_>,
    ) -> Result<ContextTokenMeasurementCapability, CoreError> {
        self.inner.input_token_measurement_capability(selection)
    }

    fn measure_input(
        &self,
        selection: ModelSelection<'_>,
        request: &ModelRequest,
        cancellation: &CancellationToken,
    ) -> Result<ContextTokenMeasurementOutcome, CoreError> {
        self.inner.measure_input(
            selection,
            &self.materialize(selection, request)?,
            cancellation,
        )
    }

    fn invoke(
        &self,
        selection: ModelSelection<'_>,
        request: &ModelRequest,
        cancellation: &CancellationToken,
    ) -> Result<ModelResponse, CoreError> {
        self.inner.invoke(
            selection,
            &self.materialize(selection, request)?,
            cancellation,
        )
    }

    fn stream(
        &self,
        selection: ModelSelection<'_>,
        request: &ModelRequest,
        cancellation: &CancellationToken,
        sink: &mut dyn ModelStreamSink,
    ) -> Result<ModelResponse, CoreError> {
        self.inner.stream(
            selection,
            &self.materialize(selection, request)?,
            cancellation,
            sink,
        )
    }
}

#[cfg(test)]
#[path = "attachment_model_service_tests.rs"]
mod tests;

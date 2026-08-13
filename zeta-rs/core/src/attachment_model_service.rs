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

    fn materialize(&self, request: &ModelRequest) -> Result<ModelRequest, CoreError> {
        let mut materialized = request.clone();
        for item in &mut materialized.input {
            let content = match item {
                InputItem::Message(message) => &mut message.content,
                InputItem::ToolResult(result) => &mut result.content,
            };
            for part in content {
                let ContentPart::ImageAttachment { attachment, detail } = part else {
                    continue;
                };
                let data_url = self
                    .attachments
                    .materialize_data_url(attachment)
                    .map_err(|error| CoreError::Context(error.to_string()))?;
                *part = ContentPart::ImageUrl {
                    url: data_url,
                    detail: *detail,
                };
            }
        }
        Ok(materialized)
    }
}

impl ModelService for AttachmentModelService {
    fn context_budget(&self, selection: ModelSelection<'_>) -> Result<ContextBudget, CoreError> {
        self.inner.context_budget(selection)
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
        self.inner
            .measure_input(selection, &self.materialize(request)?, cancellation)
    }

    fn invoke(
        &self,
        selection: ModelSelection<'_>,
        request: &ModelRequest,
        cancellation: &CancellationToken,
    ) -> Result<ModelResponse, CoreError> {
        self.inner
            .invoke(selection, &self.materialize(request)?, cancellation)
    }

    fn stream(
        &self,
        selection: ModelSelection<'_>,
        request: &ModelRequest,
        cancellation: &CancellationToken,
        sink: &mut dyn ModelStreamSink,
    ) -> Result<ModelResponse, CoreError> {
        self.inner
            .stream(selection, &self.materialize(request)?, cancellation, sink)
    }
}

#[cfg(test)]
#[path = "attachment_model_service_tests.rs"]
mod tests;

use std::sync::Arc;
use std::sync::Mutex;

use zeta_async_utils::CancellationSource;
use zeta_attachments::ImageAttachments;
use zeta_protocol::ContentPart;
use zeta_protocol::ImageDetail;
use zeta_protocol::InputItem;
use zeta_protocol::ModelRequest;
use zeta_protocol::ModelResponse;
use zeta_protocol::ResponseItem;
use zeta_protocol::StopReason;

use super::AttachmentModelService;
use crate::CoreError;
use crate::ModelSelection;
use crate::ModelService;

#[test]
fn provider_receives_ephemeral_data_url_instead_of_durable_attachment_reference() {
    let attachments = Arc::new(ImageAttachments::in_memory());
    let attachment = attachments
        .import_data_url(
            &crate::test_image::one_pixel_png_data_url(),
            ImageDetail::Auto,
        )
        .unwrap();
    let mut request = ModelRequest::text("describe this image");
    let InputItem::Message(message) = &mut request.input[0] else {
        panic!("text request must contain one message");
    };
    message.content.push(ContentPart::ImageAttachment {
        attachment,
        detail: ImageDetail::High,
    });
    let provider = Arc::new(RecordingModel::default());
    let service = AttachmentModelService::new(provider.clone(), attachments);

    service
        .invoke(
            ModelSelection::ConfiguredDefault,
            &request,
            &CancellationSource::new().token(),
        )
        .unwrap();

    assert!(matches!(
        &provider.request.lock().unwrap().as_ref().unwrap().input[0],
        InputItem::Message(message)
            if matches!(
                &message.content[1],
                ContentPart::ImageUrl { url, detail: ImageDetail::High }
                    if url.starts_with("data:image/png;base64,")
            )
    ));
    assert!(matches!(
        &request.input[0],
        InputItem::Message(message)
            if matches!(&message.content[1], ContentPart::ImageAttachment { .. })
    ));
}

#[derive(Default)]
struct RecordingModel {
    request: Mutex<Option<ModelRequest>>,
}

impl ModelService for RecordingModel {
    fn invoke(
        &self,
        _: ModelSelection<'_>,
        request: &ModelRequest,
        _: &zeta_async_utils::CancellationToken,
    ) -> Result<ModelResponse, CoreError> {
        *self.request.lock().unwrap() = Some(request.clone());
        Ok(ModelResponse {
            output: vec![ResponseItem::Text("ok".into())],
            usage: None,
            stop_reason: StopReason::Completed,
        })
    }
}

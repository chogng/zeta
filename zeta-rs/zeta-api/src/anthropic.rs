mod messages;

use crate::{ApiError, JsonHttpTransport, ModelRequest, ModelResponse, ResolvedApiTarget};

const API_VERSION: &str = "2023-06-01";

pub(crate) fn complete(
    target: &ResolvedApiTarget,
    model: &str,
    request: &ModelRequest,
    transport: &dyn JsonHttpTransport,
) -> Result<ModelResponse, ApiError> {
    messages::complete(target, model, request, API_VERSION, transport)
}

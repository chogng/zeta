mod responses;

use crate::{ApiError, JsonHttpTransport, ModelRequest, ModelResponse, ResolvedApiTarget};

pub(crate) fn complete(
    target: &ResolvedApiTarget,
    model: &str,
    request: &ModelRequest,
    transport: &dyn JsonHttpTransport,
) -> Result<ModelResponse, ApiError> {
    responses::complete(target, model, request, transport)
}

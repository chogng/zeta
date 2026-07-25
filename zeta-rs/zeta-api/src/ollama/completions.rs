use crate::{
    ApiError, JsonHttpTransport, ModelRequest, ModelResponse, ResolvedApiTarget, openai_compatible,
};

pub(crate) fn complete(
    target: &ResolvedApiTarget,
    model: &str,
    request: &ModelRequest,
    transport: &dyn JsonHttpTransport,
) -> Result<ModelResponse, ApiError> {
    openai_compatible::complete(target, model, request, transport)
}

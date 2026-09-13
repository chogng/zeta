use crate::ApiError;
use ash_http_client::HttpHeader;

#[derive(Clone, Copy)]
pub(crate) enum ResponseFormat {
    Json,
    EventStream,
}

/// Builds one unambiguous request without changing the caller's authenticated target.
pub(crate) fn build(
    input: Vec<HttpHeader>,
    format: ResponseFormat,
) -> Result<Vec<HttpHeader>, ApiError> {
    let mut headers = Vec::new();
    for header in input {
        insert(&mut headers, header.name(), header.value())?;
    }
    insert(&mut headers, "Content-Type", "application/json")?;
    insert(
        &mut headers,
        "Accept",
        match format {
            ResponseFormat::Json => "application/json",
            ResponseFormat::EventStream => "text/event-stream",
        },
    )?;
    Ok(headers)
}

pub(crate) fn insert(
    headers: &mut Vec<HttpHeader>,
    name: &str,
    value: &str,
) -> Result<(), ApiError> {
    HttpHeader::new(name, value)
        .validate()
        .map_err(|_| ApiError::InvalidRequest("invalid API request header".into()))?;
    if let Some(existing) = headers
        .iter()
        .find(|header| header.name().eq_ignore_ascii_case(name))
    {
        if existing.value() != value {
            return Err(ApiError::InvalidRequest(format!(
                "conflicting API request header: {name}"
            )));
        }
    } else {
        headers.push(HttpHeader::new(name, value));
    }
    Ok(())
}

use crate::FontCatalogError;

pub(crate) fn system_family_names() -> Result<Vec<String>, FontCatalogError> {
    let collection = coretext::FontCollection::available()
        .map_err(|error| FontCatalogError::Backend(error.to_string()))?;
    Ok(collection
        .matching_descriptors()
        .into_iter()
        .filter_map(|descriptor| descriptor.family_name())
        .collect())
}

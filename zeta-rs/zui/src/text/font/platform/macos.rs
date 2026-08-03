use crate::text::FontCatalogError;

pub(crate) fn configure_font_database(database: &mut cosmic_text::fontdb::Database) {
    let unsupported_faces = database
        .faces()
        .filter(|face| face.post_script_name == "GB18030Bitmap")
        .map(|face| face.id)
        .collect::<Vec<_>>();
    for face_id in unsupported_faces {
        database.remove_face(face_id);
    }
}

pub(crate) fn system_family_names() -> Result<Vec<String>, FontCatalogError> {
    let collection = coretext::FontCollection::available()
        .map_err(|error| FontCatalogError::Backend(error.to_string()))?;
    Ok(collection
        .matching_descriptors()
        .into_iter()
        .filter_map(|descriptor| descriptor.family_name())
        .collect())
}

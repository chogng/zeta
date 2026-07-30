use crate::FontCatalogError;

pub(crate) fn configure_font_database(_database: &mut glyphon::fontdb::Database) {}

pub(crate) fn system_family_names() -> Result<Vec<String>, FontCatalogError> {
    let font_system = glyphon::FontSystem::new();
    Ok(font_system
        .db()
        .faces()
        .flat_map(|face| face.families.iter().map(|family| family.0.clone()))
        .collect())
}

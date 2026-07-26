use std::path::Path;
use zeta_app_server_protocol::{JSON_SCHEMA_FIXTURE, TYPESCRIPT_FIXTURE, json_schema, typescript};

fn main() {
    let manifest_directory = Path::new(env!("CARGO_MANIFEST_DIR"));
    write_fixture(&manifest_directory.join(JSON_SCHEMA_FIXTURE), json_schema());
    write_fixture(&manifest_directory.join(TYPESCRIPT_FIXTURE), typescript());
}

fn write_fixture(path: &Path, contents: String) {
    let parent = path
        .parent()
        .expect("schema fixtures must have a parent directory");
    std::fs::create_dir_all(parent).unwrap_or_else(|error| {
        panic!(
            "failed to create schema fixture directory {}: {error}",
            parent.display()
        )
    });
    std::fs::write(path, contents).unwrap_or_else(|error| {
        panic!("failed to write schema fixture {}: {error}", path.display())
    });
}

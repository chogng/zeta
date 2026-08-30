use super::TerminalSettings;

const CURRENT_SCHEMA_VERSION: u64 = 1;
// Raise this only when the product support window no longer includes the removed versions.
const MIN_SUPPORTED_SCHEMA_VERSION: u64 = 1;

pub(super) struct DecodedSettings {
    pub(super) settings: TerminalSettings,
    pub(super) rewrite_required: bool,
}

pub(super) fn decode(contents: &[u8]) -> Result<DecodedSettings, String> {
    let mut value = serde_json::from_slice::<serde_json::Value>(contents)
        .map_err(|error| format!("invalid JSON: {error}"))?;
    let root = value
        .as_object_mut()
        .ok_or_else(|| "terminal settings root must be a JSON object".to_owned())?;
    let version = root.remove("schemaVersion");
    let rewrite_required = match version {
        None => {
            migrate_unversioned(root)?;
            true
        }
        Some(serde_json::Value::Number(version)) => {
            let version = version.as_u64().ok_or_else(|| {
                "terminal settings schemaVersion must be a non-negative integer".to_owned()
            })?;
            validate_version(version)?;
            false
        }
        Some(_) => {
            return Err("terminal settings schemaVersion must be a non-negative integer".into());
        }
    };
    let settings = serde_json::from_value::<TerminalSettings>(value)
        .map_err(|error| format!("invalid JSON: {error}"))?
        .validate()?;
    Ok(DecodedSettings {
        settings,
        rewrite_required,
    })
}

pub(super) fn encode(settings: &TerminalSettings) -> Result<Vec<u8>, String> {
    let mut value = serde_json::to_value(settings)
        .map_err(|error| format!("could not serialize terminal settings: {error}"))?;
    let fields = value
        .as_object_mut()
        .ok_or_else(|| "terminal settings did not serialize as a JSON object".to_owned())?;
    let mut root = serde_json::Map::new();
    root.insert(
        "schemaVersion".into(),
        serde_json::Value::Number(CURRENT_SCHEMA_VERSION.into()),
    );
    root.append(fields);
    let mut contents = serde_json::to_vec_pretty(&serde_json::Value::Object(root))
        .map_err(|error| format!("could not serialize terminal settings: {error}"))?;
    contents.push(b'\n');
    Ok(contents)
}

fn validate_version(version: u64) -> Result<(), String> {
    if version > CURRENT_SCHEMA_VERSION {
        return Err(format!(
            "terminal settings schema version {version} is newer than supported version {CURRENT_SCHEMA_VERSION}"
        ));
    }
    if version < MIN_SUPPORTED_SCHEMA_VERSION {
        return Err(format!(
            "terminal settings schema version {version} is older than minimum supported version {MIN_SUPPORTED_SCHEMA_VERSION}"
        ));
    }
    Ok(())
}

fn migrate_unversioned(
    root: &mut serde_json::Map<String, serde_json::Value>,
) -> Result<(), String> {
    let Some(permissions) = root.remove("additionalDirectoryPermissions") else {
        return Ok(());
    };
    if root.contains_key("dirPermissions") {
        return Err(
            "terminal settings contain both additionalDirectoryPermissions and dirPermissions"
                .into(),
        );
    }
    let serde_json::Value::Object(mut permissions) = permissions else {
        return Err(
            "terminal settings additionalDirectoryPermissions must be a JSON object".into(),
        );
    };
    for (old, current) in [
        ("watchFileChanges", "watchFiles"),
        ("useWorkspaceFiles", "browseFiles"),
        ("useWorkspaceSearch", "searchFiles"),
        ("loadInstructionsAndAgents", "loadInstructions"),
    ] {
        if let Some(value) = permissions.remove(old) {
            if permissions.insert(current.into(), value).is_some() {
                return Err(format!(
                    "terminal settings contain both legacy field {old} and current field {current}"
                ));
            }
        }
    }
    root.insert("dirPermissions".into(), permissions.into());
    Ok(())
}

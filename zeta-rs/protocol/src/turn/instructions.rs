use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::error::Error;
use std::fmt;
use ts_rs::TS;

/// Immutable instructions selected before one Agent Turn is durably accepted.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TurnInstructions {
    owner: String,
    id: String,
    revision: String,
    body: String,
}

impl TurnInstructions {
    /// Creates and validates one frozen instruction snapshot.
    pub fn new(
        owner: impl Into<String>,
        id: impl Into<String>,
        revision: impl Into<String>,
        body: impl Into<String>,
    ) -> Result<Self, InvalidTurnInstructions> {
        let instructions = Self {
            owner: owner.into(),
            id: id.into(),
            revision: revision.into(),
            body: body.into(),
        };
        instructions.validate()?;
        Ok(instructions)
    }

    /// Validates values received through a serialized protocol boundary.
    pub fn validate(&self) -> Result<(), InvalidTurnInstructions> {
        if self.owner.trim().is_empty() {
            return Err(InvalidTurnInstructions("owner"));
        }
        if self.id.trim().is_empty() {
            return Err(InvalidTurnInstructions("id"));
        }
        if self.revision.trim().is_empty() {
            return Err(InvalidTurnInstructions("revision"));
        }
        if self.body.trim().is_empty() {
            return Err(InvalidTurnInstructions("body"));
        }
        Ok(())
    }

    /// Returns the capability crate that owns these instructions.
    pub fn owner(&self) -> &str {
        &self.owner
    }

    /// Returns the stable logical prompt identifier.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the authored semantic revision.
    pub fn revision(&self) -> &str {
        &self.revision
    }

    /// Returns the exact model-facing instruction body.
    pub fn body(&self) -> &str {
        &self.body
    }
}

/// Identifies an invalid field in a frozen Turn instruction snapshot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidTurnInstructions(&'static str);

impl fmt::Display for InvalidTurnInstructions {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Turn instructions {} must not be empty", self.0)
    }
}

impl Error for InvalidTurnInstructions {}

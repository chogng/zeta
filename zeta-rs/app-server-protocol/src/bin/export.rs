use std::io::{self, Write};
use zeta_app_server_protocol::{json_schema, typescript};

enum Artifact {
    JsonSchema,
    TypeScript,
}

impl Artifact {
    fn parse(argument: Option<String>) -> Result<Self, String> {
        match argument.as_deref() {
            Some("json") => Ok(Self::JsonSchema),
            Some("typescript") => Ok(Self::TypeScript),
            _ => Err("usage: export <json|typescript>".into()),
        }
    }

    fn contents(self) -> String {
        match self {
            Self::JsonSchema => json_schema(),
            Self::TypeScript => typescript(),
        }
    }
}

fn main() {
    let artifact = match Artifact::parse(std::env::args().nth(1)) {
        Ok(artifact) if std::env::args().nth(2).is_none() => artifact,
        Ok(_) | Err(_) => {
            eprintln!("usage: export <json|typescript>");
            std::process::exit(2);
        }
    };

    if let Err(error) = io::stdout().write_all(artifact.contents().as_bytes()) {
        eprintln!("export: {error}");
        std::process::exit(1);
    }
}

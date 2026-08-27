use std::path::{Path, PathBuf};
use zeta_app_server_protocol::{json_schema, typescript};

const USAGE: &str = "usage: generate_protocol <json|typescript> --out <directory>";

enum Artifact {
    JsonSchema,
    TypeScript,
}

impl Artifact {
    fn parse(argument: &str) -> Result<Self, String> {
        match argument {
            "json" => Ok(Self::JsonSchema),
            "typescript" => Ok(Self::TypeScript),
            _ => Err(USAGE.into()),
        }
    }

    fn file_name(&self) -> &'static str {
        match self {
            Self::JsonSchema => "schema.json",
            Self::TypeScript => "types.ts",
        }
    }

    fn contents(&self) -> String {
        match self {
            Self::JsonSchema => json_schema(),
            Self::TypeScript => typescript(),
        }
    }
}

struct Command {
    artifact: Artifact,
    output_directory: PathBuf,
}

impl Command {
    fn parse(arguments: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut arguments = arguments.into_iter();
        let artifact = arguments
            .next()
            .ok_or_else(|| USAGE.to_owned())
            .and_then(|argument| Artifact::parse(&argument))?;
        if arguments.next().as_deref() != Some("--out") {
            return Err(USAGE.into());
        }
        let output_directory = arguments
            .next()
            .filter(|path| !path.is_empty())
            .map(PathBuf::from)
            .ok_or_else(|| USAGE.to_owned())?;
        if arguments.next().is_some() {
            return Err(USAGE.into());
        }

        Ok(Self {
            artifact,
            output_directory,
        })
    }

    fn write(self) -> std::io::Result<()> {
        write_artifact(
            &self.output_directory,
            self.artifact.file_name(),
            self.artifact.contents(),
        )
    }
}

fn main() {
    let command = match Command::parse(std::env::args().skip(1)) {
        Ok(command) => command,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };

    if let Err(error) = command.write() {
        eprintln!("generate_protocol: {error}");
        std::process::exit(1);
    }
}

fn write_artifact(directory: &Path, file_name: &str, contents: String) -> std::io::Result<()> {
    std::fs::create_dir_all(directory)?;
    std::fs::write(directory.join(file_name), contents)
}

#[cfg(test)]
mod tests {
    use super::{Artifact, Command};
    use std::path::PathBuf;

    #[test]
    fn parses_a_typescript_output_directory() {
        let command = Command::parse([
            "typescript".to_owned(),
            "--out".to_owned(),
            "generated/app-server".to_owned(),
        ])
        .unwrap();

        assert!(matches!(command.artifact, Artifact::TypeScript));
        assert_eq!(
            command.output_directory,
            PathBuf::from("generated/app-server")
        );
    }

    #[test]
    fn rejects_missing_and_extra_arguments() {
        assert!(Command::parse(["json".to_owned()]).is_err());
        assert!(
            Command::parse([
                "json".to_owned(),
                "--out".to_owned(),
                "schema".to_owned(),
                "unexpected".to_owned(),
            ])
            .is_err()
        );
    }
}

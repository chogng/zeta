use std::path::Path;
use std::path::PathBuf;
use zeta_app_server_protocol::JSON_SCHEMA_FIXTURE;
use zeta_app_server_protocol::TYPESCRIPT_FIXTURE_DIRECTORY;
use zeta_app_server_protocol::json_schema;
use zeta_app_server_protocol::typescript_files;

const USAGE: &str = "usage: generate_protocol <json|typescript> --out <directory>\n       generate_protocol fixtures";

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
}

enum Command {
    Generate {
        artifact: Artifact,
        output_directory: PathBuf,
    },
    WriteFixtures,
}

impl Command {
    fn parse(arguments: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut arguments = arguments.into_iter();
        let operation = arguments.next().ok_or_else(|| USAGE.to_owned())?;
        if operation == "fixtures" {
            return if arguments.next().is_none() {
                Ok(Self::WriteFixtures)
            } else {
                Err(USAGE.into())
            };
        }

        let artifact = Artifact::parse(&operation)?;
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

        Ok(Self::Generate {
            artifact,
            output_directory,
        })
    }

    fn write(self) -> std::io::Result<()> {
        match self {
            Self::Generate {
                artifact,
                output_directory,
            } => match artifact {
                Artifact::JsonSchema => {
                    write_artifact(&output_directory, "schema.json", json_schema())
                }
                Artifact::TypeScript => write_typescript_files(&output_directory),
            },
            Self::WriteFixtures => write_fixtures(),
        }
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

fn write_fixtures() -> std::io::Result<()> {
    let crate_directory = Path::new(env!("CARGO_MANIFEST_DIR"));
    write_fixture(crate_directory.join(JSON_SCHEMA_FIXTURE), json_schema())?;
    write_typescript_files(&crate_directory.join(TYPESCRIPT_FIXTURE_DIRECTORY))
}

fn write_typescript_files(directory: &Path) -> std::io::Result<()> {
    for (file_name, contents) in typescript_files() {
        write_artifact(directory, file_name, contents)?;
    }
    Ok(())
}

fn write_fixture(path: PathBuf, contents: String) -> std::io::Result<()> {
    if let Some(directory) = path.parent() {
        std::fs::create_dir_all(directory)?;
    }
    std::fs::write(path, contents)
}

#[cfg(test)]
mod tests {
    use super::Artifact;
    use super::Command;
    use std::path::PathBuf;

    #[test]
    fn parses_a_typescript_output_directory() {
        let command = Command::parse([
            "typescript".to_owned(),
            "--out".to_owned(),
            "generated/app-server".to_owned(),
        ])
        .unwrap();

        let Command::Generate {
            artifact,
            output_directory,
        } = command
        else {
            panic!("expected an artifact generation command");
        };
        assert!(matches!(artifact, Artifact::TypeScript));
        assert_eq!(output_directory, PathBuf::from("generated/app-server"));
    }

    #[test]
    fn parses_the_checked_in_fixture_command() {
        assert!(matches!(
            Command::parse(["fixtures".to_owned()]),
            Ok(Command::WriteFixtures)
        ));
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
        assert!(Command::parse(["fixtures".to_owned(), "unexpected".to_owned()]).is_err());
    }
}

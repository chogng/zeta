use crate::Result;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncRead;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

const OUTPUT_LIMIT: u64 = 8 * 1024 * 1024;

pub(super) async fn run(
    executable: &Path,
    arguments: &[String],
    input: Option<&[u8]>,
) -> Result<Vec<u8>> {
    let mut child = Command::new(executable)
        .args(arguments)
        .env("GH_PROMPT_DISABLED", "1")
        .env("GH_PAGER", "cat")
        .env_remove("GH_REPO")
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|error| format!("Cannot start GitHub CLI; install gh and sign in: {error}"))?;
    let stdout = child.stdout.take().ok_or("Missing GitHub stdout")?;
    let stderr = child.stderr.take().ok_or("Missing GitHub stderr")?;
    let stdin = child.stdin.take();
    let operation = async {
        let write = async {
            if let (Some(mut stdin), Some(input)) = (stdin, input) {
                stdin
                    .write_all(input)
                    .await
                    .map_err(|error| error.to_string())?;
                stdin.shutdown().await.map_err(|error| error.to_string())?;
            }
            Ok::<_, String>(())
        };
        let (_, stdout, stderr, status) =
            tokio::try_join!(write, read(stdout), read(stderr), async {
                child.wait().await.map_err(|error| error.to_string())
            })?;
        if !status.success() {
            return Err(format!(
                "GitHub request failed: {}",
                String::from_utf8_lossy(&stderr).trim()
            ));
        }
        Ok(stdout)
    };
    match tokio::time::timeout(Duration::from_secs(30), operation).await {
        Ok(Ok(output)) => Ok(output),
        outcome => {
            let _ = child.kill().await;
            match outcome {
                Ok(Err(error)) => Err(error),
                _ => Err("GitHub request timed out".into()),
            }
        }
    }
}

async fn read(reader: impl AsyncRead + Unpin) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader
        .take(OUTPUT_LIMIT + 1)
        .read_to_end(&mut bytes)
        .await
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 > OUTPUT_LIMIT {
        return Err("GitHub response exceeds 8 MiB".into());
    }
    Ok(bytes)
}

use crate::{ConfigChange, ConfigError, ConfigGeneration, ConfigRevision};
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

pub(crate) fn start(
    mut connection: Connection,
    config_path: PathBuf,
    shutdown: Receiver<()>,
    subscribers: Arc<Mutex<Vec<Sender<ConfigChange>>>>,
    last_published: Arc<Mutex<ConfigChange>>,
) -> Result<JoinHandle<()>, ConfigError> {
    let mut data_version = sqlite_data_version(&connection)?;
    std::thread::Builder::new()
        .name("zeta-config-sqlite".into())
        .spawn(move || {
            loop {
                match shutdown.recv_timeout(Duration::from_millis(100)) {
                    Ok(()) | Err(RecvTimeoutError::Disconnected) => break,
                    Err(RecvTimeoutError::Timeout) => {}
                }
                if let Ok(Some(change)) =
                    crate::store::reconcile_external_change(&mut connection, &config_path)
                {
                    publish(&subscribers, &last_published, change);
                }
                let Ok(next_data_version) = sqlite_data_version(&connection) else {
                    continue;
                };
                if next_data_version == data_version {
                    continue;
                }
                let Ok(change) = read_change(&connection) else {
                    continue;
                };
                data_version = next_data_version;
                publish(&subscribers, &last_published, change);
            }
        })
        .map_err(|error| ConfigError(error.to_string()))
}

pub(crate) fn publish(
    subscribers: &Mutex<Vec<Sender<ConfigChange>>>,
    last_published: &Mutex<ConfigChange>,
    change: ConfigChange,
) {
    let mut last = last_published
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if *last == change {
        return;
    }
    *last = change;
    subscribers
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .retain(|subscriber| subscriber.send(change).is_ok());
}

fn read_change(connection: &Connection) -> Result<ConfigChange, ConfigError> {
    let (revision, generation) = connection
        .query_row(
            "SELECT revision, generation FROM config_metadata WHERE authority_id = 1",
            [],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .map_err(sql_error)?;
    Ok(ConfigChange {
        revision: ConfigRevision::new(from_sql_integer(revision)?),
        generation: ConfigGeneration::new(from_sql_integer(generation)?),
    })
}

fn sqlite_data_version(connection: &Connection) -> Result<i64, ConfigError> {
    connection
        .query_row("PRAGMA data_version", [], |row| row.get(0))
        .map_err(sql_error)
}

fn from_sql_integer(value: i64) -> Result<u64, ConfigError> {
    u64::try_from(value).map_err(|_| ConfigError("negative configuration revision".into()))
}

fn sql_error(error: impl std::fmt::Display) -> ConfigError {
    ConfigError(format!("config SQLite monitor error: {error}"))
}

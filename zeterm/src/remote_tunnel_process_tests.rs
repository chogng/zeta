use std::num::NonZeroU16;
use std::path::Path;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::Duration;
use std::time::Instant;

use tempfile::TempDir;
use zeta_remote::SshHost;

use super::RemoteTunnelTarget;
use super::RemoteTunnelUpdate;
use super::recovery_delay;
use super::spawn_remote_tunnel;

#[cfg(unix)]
const TEST_EVENT_TIMEOUT: Duration = Duration::from_secs(5);

#[cfg(unix)]
#[test]
fn native_supervisor_reports_ready_arguments_and_stops_its_child() {
    use crate::launch_test_support::make_executable;

    let directory = TempDir::new().unwrap();
    let fake_ssh = directory.path().join("fake-ssh");
    let arguments_path = directory.path().join("arguments.txt");
    std::fs::write(
        &fake_ssh,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > '{}'\nexec sleep 60\n",
            arguments_path.display()
        ),
    )
    .unwrap();
    make_executable(&fake_ssh);
    let listener = FakeTunnelListener::start(arguments_path.clone());
    let (sender, receiver) = mpsc::channel();
    let process = spawn_remote_tunnel(
        RemoteTunnelTarget {
            host: SshHost::parse("build.example").unwrap(),
            ssh_executable: fake_ssh,
        },
        NonZeroU16::new(3_000).unwrap(),
        move |event| {
            let _ = sender.send(event);
        },
    )
    .unwrap();

    let ready = receiver.recv_timeout(TEST_EVENT_TIMEOUT).unwrap();
    let local_port = match ready.update() {
        RemoteTunnelUpdate::Ready { local_port } => *local_port,
        update => panic!("unexpected startup update: {update:?}"),
    };
    let arguments = read_when_available(&arguments_path);
    assert_eq!(
        arguments,
        format!(
            "-N\n-T\n-o\nBatchMode=yes\n-o\nExitOnForwardFailure=yes\n-o\nConnectTimeout=10\n-L\n127.0.0.1:{local_port}:127.0.0.1:3000\nbuild.example\n"
        )
    );

    drop(process);
    assert_eq!(
        receiver.recv_timeout(TEST_EVENT_TIMEOUT).unwrap().update(),
        &RemoteTunnelUpdate::Stopped
    );
    drop(listener);
}

#[cfg(unix)]
fn read_when_available(path: &Path) -> String {
    let deadline = Instant::now() + TEST_EVENT_TIMEOUT;
    loop {
        match std::fs::read_to_string(path) {
            Ok(contents) if !contents.is_empty() => return contents,
            Ok(_) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Ok(contents) => return contents,
            Err(error)
                if error.kind() == std::io::ErrorKind::NotFound && Instant::now() < deadline =>
            {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("could not read fake OpenSSH arguments: {error}"),
        }
    }
}

#[cfg(unix)]
#[test]
fn native_supervisor_does_not_publish_an_early_openssh_exit() {
    let (sender, receiver) = mpsc::channel();
    let _process = spawn_remote_tunnel(
        RemoteTunnelTarget {
            host: SshHost::parse("build.example").unwrap(),
            ssh_executable: "/usr/bin/false".into(),
        },
        NonZeroU16::new(3_000).unwrap(),
        move |event| {
            let _ = sender.send(event);
        },
    )
    .unwrap();

    let event = receiver.recv_timeout(TEST_EVENT_TIMEOUT).unwrap();
    assert!(
        matches!(event.update(), RemoteTunnelUpdate::Failed(error) if error.contains("before it became ready"))
    );
}

#[test]
fn recovery_backoff_is_exponential_and_bounded() {
    assert_eq!(recovery_delay(0), Duration::from_millis(250));
    assert_eq!(recovery_delay(1), Duration::from_millis(500));
    assert_eq!(recovery_delay(2), Duration::from_secs(1));
    assert_eq!(recovery_delay(3), Duration::from_secs(2));
    assert_eq!(recovery_delay(30), Duration::from_secs(2));
}

#[cfg(unix)]
#[test]
fn native_supervisor_recovers_on_the_same_local_port() {
    use crate::launch_test_support::make_executable;

    let directory = TempDir::new().unwrap();
    let fake_ssh = directory.path().join("fake-ssh");
    std::fs::write(
        &fake_ssh,
        "#!/bin/sh\n\
directory=$(dirname \"$0\")\n\
count_path=\"$directory/invocation-count\"\n\
count=0\n\
if [ -f \"$count_path\" ]; then count=$(cat \"$count_path\"); fi\n\
count=$((count + 1))\n\
printf '%s' \"$count\" > \"$count_path\"\n\
printf '%s\\n' \"$@\" > \"$directory/arguments-$count.txt\"\n\
if [ \"$count\" -eq 1 ]; then sleep 0.15; exit 17; fi\n\
exec sleep 60\n",
    )
    .unwrap();
    make_executable(&fake_ssh);
    let listener = FakeTunnelListener::start(directory.path().join("arguments-1.txt"));
    let (sender, receiver) = mpsc::channel();
    let process = spawn_remote_tunnel(
        RemoteTunnelTarget {
            host: SshHost::parse("build.example").unwrap(),
            ssh_executable: fake_ssh,
        },
        NonZeroU16::new(3_000).unwrap(),
        move |event| {
            let _ = sender.send(event);
        },
    )
    .unwrap();

    let first_ready = receiver.recv_timeout(TEST_EVENT_TIMEOUT).unwrap();
    let local_port = match first_ready.update() {
        RemoteTunnelUpdate::Ready { local_port } => *local_port,
        update => panic!("unexpected first update: {update:?}"),
    };
    assert_eq!(
        receiver.recv_timeout(TEST_EVENT_TIMEOUT).unwrap().update(),
        &RemoteTunnelUpdate::Recovering { attempt: 1 }
    );
    assert_eq!(
        receiver.recv_timeout(TEST_EVENT_TIMEOUT).unwrap().update(),
        &RemoteTunnelUpdate::Ready { local_port }
    );

    let first_arguments = read_when_available(&directory.path().join("arguments-1.txt"));
    let second_arguments = read_when_available(&directory.path().join("arguments-2.txt"));
    let forward = format!("127.0.0.1:{local_port}:127.0.0.1:3000");
    assert!(first_arguments.lines().any(|argument| argument == forward));
    assert!(second_arguments.lines().any(|argument| argument == forward));

    drop(process);
    assert_eq!(
        receiver.recv_timeout(TEST_EVENT_TIMEOUT).unwrap().update(),
        &RemoteTunnelUpdate::Stopped
    );
    drop(listener);
}

#[cfg(unix)]
#[test]
fn cancelling_recovery_interrupts_the_backoff() {
    use crate::launch_test_support::make_executable;

    let directory = TempDir::new().unwrap();
    let fake_ssh = directory.path().join("fake-ssh");
    std::fs::write(
        &fake_ssh,
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"${0%/*}/arguments.txt\"\nsleep 0.15\nexit 17\n",
    )
    .unwrap();
    make_executable(&fake_ssh);
    let listener = FakeTunnelListener::start(directory.path().join("arguments.txt"));
    let (sender, receiver) = mpsc::channel();
    let process = spawn_remote_tunnel(
        RemoteTunnelTarget {
            host: SshHost::parse("build.example").unwrap(),
            ssh_executable: fake_ssh,
        },
        NonZeroU16::new(3_000).unwrap(),
        move |event| {
            let _ = sender.send(event);
        },
    )
    .unwrap();

    assert!(matches!(
        receiver.recv_timeout(TEST_EVENT_TIMEOUT).unwrap().update(),
        RemoteTunnelUpdate::Ready { .. }
    ));
    assert_eq!(
        receiver.recv_timeout(TEST_EVENT_TIMEOUT).unwrap().update(),
        &RemoteTunnelUpdate::Recovering { attempt: 1 }
    );
    let started = Instant::now();
    drop(process);
    assert!(started.elapsed() < Duration::from_millis(500));
    assert_eq!(
        receiver.recv_timeout(TEST_EVENT_TIMEOUT).unwrap().update(),
        &RemoteTunnelUpdate::Stopped
    );
    drop(listener);
}

#[cfg(unix)]
struct FakeTunnelListener {
    stop: mpsc::Sender<()>,
    worker: Option<JoinHandle<()>>,
}

#[cfg(unix)]
impl FakeTunnelListener {
    fn start(arguments_path: PathBuf) -> Self {
        let (stop, receiver) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            let arguments = read_when_available(&arguments_path);
            let local_port = forwarded_local_port(&arguments);
            let _listener = std::net::TcpListener::bind(("127.0.0.1", local_port)).unwrap();
            let _ = receiver.recv();
        });
        Self {
            stop,
            worker: Some(worker),
        }
    }
}

#[cfg(unix)]
impl Drop for FakeTunnelListener {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        if let Some(worker) = self.worker.take() {
            if std::thread::panicking() {
                let _ = worker.join();
            } else {
                worker.join().unwrap();
            }
        }
    }
}

#[cfg(unix)]
fn forwarded_local_port(arguments: &str) -> u16 {
    let mut arguments = arguments.lines();
    while let Some(argument) = arguments.next() {
        if argument == "-L" {
            return arguments
                .next()
                .and_then(|forward| forward.split(':').nth(1))
                .and_then(|port| port.parse().ok())
                .expect("fake OpenSSH should receive a loopback forward");
        }
    }
    panic!("fake OpenSSH did not receive -L");
}

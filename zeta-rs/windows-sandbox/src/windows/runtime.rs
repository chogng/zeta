// Licensed under the MIT License.
//! Explicit, journaled provisioning and exclusive execution-identity leases.

use super::account;
use super::account::Account;
use super::account::NetworkMode;
use super::network::Rules;
use super::win;
use super::win::Result;
use serde::Deserialize;
use serde::Serialize;
use sha2::Digest;
use std::fs::File;
use std::fs::OpenOptions;
use std::os::windows::fs::OpenOptionsExt;
use std::path::Path;
use std::path::PathBuf;
use windows_sys::Win32::Security::DACL_SECURITY_INFORMATION;
use windows_sys::Win32::Security::PROTECTED_DACL_SECURITY_INFORMATION;
use windows_sys::Win32::Security::SetFileSecurityW;

const VERSION: u32 = 1;

pub(super) fn device_sid() -> Result<String> {
    let digest = sha2::Sha256::digest(format!(
        "zeta-windows-device-v{VERSION}:{}",
        win::current_user()?
    ));
    let parts = digest[..16]
        .chunks_exact(4)
        .map(|bytes| u32::from_le_bytes(bytes.try_into().unwrap()).to_string())
        .collect::<Vec<_>>();
    Ok(format!("S-1-5-21-{}", parts.join("-")))
}

#[derive(Clone, Copy, Deserialize, Serialize, Eq, PartialEq)]
enum Status {
    Preparing,
    Ready,
    Removing,
}

#[derive(Deserialize, Serialize)]
struct State {
    version: u32,
    owner: String,
    status: Status,
    runner_hash: String,
    device_sid: String,
    devices: Vec<String>,
    accounts: Vec<Account>,
    rules: Option<Rules>,
}

impl State {
    fn save(&self, root: &Path) -> Result<()> {
        let mut json = serde_json::to_vec(self).map_err(|_| "could not encode runtime journal")?;
        let result = account::seal(&root.join("state.dpapi"), &mut json);
        for byte in &mut json {
            unsafe {
                std::ptr::write_volatile(byte, 0);
            }
        }
        result
    }
}

pub(super) struct Lease {
    pub(super) account: Account,
    pub(super) root: PathBuf,
    pub(super) runner: PathBuf,
    pub(super) device_sid: String,
    pub(super) runner_hash: String,
    _lock: File,
    _pins: Vec<win::Handle>,
}

pub(super) fn root() -> Result<PathBuf> {
    Ok(win::program_data()?.join(format!("Zeta-Sandbox-{}", win::current_user()?)))
}

pub(super) fn hash(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
    Ok(format!("{:x}", sha2::Sha256::digest(bytes)))
}

fn read_state() -> Result<(PathBuf, State)> {
    let root = root()?;
    let mut data = account::unseal(&root.join("state.dpapi"))
        .map_err(|_| "the Zeta user runtime requires setup; use zeta-windows-sandbox setup with explicit account and network authorization")?;
    let parsed =
        serde_json::from_slice::<State>(&data).map_err(|_| "invalid Zeta user runtime state");
    for byte in &mut data {
        unsafe {
            std::ptr::write_volatile(byte, 0);
        }
    }
    let state = parsed?;
    if state.version != VERSION
        || state.owner != win::current_user()?
        || state.accounts.is_empty()
        || state.accounts.len() > 48
    {
        return Err("the runtime journal has an unexpected owner, version, or account set".into());
    }
    Ok((root, state))
}

fn checked_state() -> Result<(PathBuf, State, Vec<win::Handle>)> {
    let (root, state) = read_state()?;
    if state.status != Status::Ready {
        return Err("runtime provisioning is incomplete; run explicit removal before setup".into());
    }
    super::devices::verify(&state.device_sid)?;
    let runner = root.join("bin").join("zeta-windows-sandbox.exe");
    let pins = win::pin_executable(&runner)?;
    if hash(&runner)? != state.runner_hash {
        return Err("the Zeta runtime executable changed; setup is required".into());
    }
    state
        .rules
        .as_ref()
        .ok_or("runtime network plan is missing")?
        .verify(&state.accounts)?;
    Ok((root, state, pins))
}

pub(super) fn available(mode: NetworkMode) -> Result<String> {
    let (_, state, _pins) = checked_state()?;
    if state.accounts.iter().any(|account| account.mode == mode) {
        Ok(state.runner_hash)
    } else {
        Err("the requested network mode was not provisioned".into())
    }
}

pub(super) fn lease(mode: NetworkMode) -> Result<Lease> {
    // Serialize lease acquisition with removal, including callers that already
    // passed an earlier availability check.
    let _setup_lock = OpenOptions::new()
        .read(true)
        .share_mode(windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ)
        .open(root()?.join("setup.lock"))
        .map_err(|error| format!("runtime provisioning or removal is in progress: {error}"))?;
    let (root, state, pins) = checked_state()?;
    for (index, account) in state.accounts.into_iter().enumerate() {
        if account.mode != mode {
            continue;
        }
        let lock = match OpenOptions::new()
            .read(true)
            .write(true)
            .share_mode(0)
            .open(root.join(format!("lease-{index}")))
        {
            Ok(lock) => lock,
            Err(error) if error.raw_os_error() == Some(32) => continue,
            Err(error) => return Err(error.to_string()),
        };
        if account::lookup(&account.name)? != account.sid {
            return Err("a provisioned sandbox account has been replaced".into());
        }
        if root
            .join("runs")
            .join(&account.sid)
            .try_exists()
            .map_err(|error| error.to_string())?
        {
            return Err("a previous execution needs explicit installation recovery before this identity can be reused".into());
        }
        return Ok(Lease {
            account,
            runner: root.join("bin").join("zeta-windows-sandbox.exe"),
            device_sid: state.device_sid.clone(),
            runner_hash: state.runner_hash.clone(),
            root,
            _lock: lock,
            _pins: pins,
        });
    }
    Err("all independently isolated execution identities are in use".into())
}

pub(super) fn setup_plan(slots: usize) -> Result<serde_json::Value> {
    if slots == 0 || slots > 16 {
        return Err("slots must be between 1 and 16 per network mode".into());
    }
    Ok(serde_json::json!({
        "operation": "setup", "version": VERSION, "ownerSid": win::current_user()?,
        "runtimeDirectory": root()?,
        "runnerSha256": hash(&std::env::current_exe().map_err(|error| error.to_string())?)?,
        "accounts": { "namePrefix": "zeta", "slotsPerNetworkMode": slots, "total": slots * 3 },
        "network": { "modes": ["denied", "managed", "allowed"], "persistentFilters": slots * 13, "managedEndpoint": "one exclusive IPv4 loopback TCP port per managed account" },
        "filesystemAclChanges": "new runtime directory and its contents only",
        "deviceAclChanges": { "paths": super::devices::PATHS, "access": "read and write for the installation's device SID only" },
        "executionAclAuthority": "separate scoped authorization required for each command"
    }))
}

pub(super) fn removal_plan() -> Result<serde_json::Value> {
    let (root, state) = read_state()?;
    Ok(serde_json::json!({
        "operation": "remove", "version": VERSION, "ownerSid": state.owner,
        "runtimeDirectory": root, "runnerSha256": state.runner_hash,
        "accounts": state.accounts.iter().map(|account| serde_json::json!({ "name": account.name, "sid": account.sid, "ownershipTag": account.tag })).collect::<Vec<_>>(),
        "networkObjects": state.rules, "deviceSid": state.device_sid, "devices": super::devices::PATHS,
        "filesystemAclChanges": "restore this installation's recorded execution ACL changes before deleting its runtime"
    }))
}

pub(super) fn plan_digest(plan: &serde_json::Value) -> Result<String> {
    let bytes = serde_json::to_vec(plan).map_err(|error| error.to_string())?;
    Ok(format!("{:x}", sha2::Sha256::digest(bytes)))
}

pub(super) fn print_plan(plan: serde_json::Value) -> Result<()> {
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({ "sha256": plan_digest(&plan)?, "changes": plan })
        )
        .map_err(|error| error.to_string())?
    );
    Ok(())
}

pub(super) fn setup(slots: usize, approved: &str) -> Result<()> {
    if approved != plan_digest(&setup_plan(slots)?)? {
        return Err("installation approval does not match this user, binary, location, and account/network plan".into());
    }
    require_administrator()?;
    let root = root()?;
    let owner = win::current_user()?;
    if root.try_exists().map_err(|error| error.to_string())? {
        let (_, mut state) = read_state()?;
        if state.status != Status::Ready {
            return Err(
                "an incomplete setup journal exists; remove its recorded objects before setup"
                    .into(),
            );
        }
        let _setup_lock = setup_lock(&root)?;
        if state.runner_hash != hash(&std::env::current_exe().map_err(|error| error.to_string())?)?
        {
            return Err("remove the previous installation with its approved plan before installing this executable".into());
        }
        install_devices(&mut state, &root)?;
        available(NetworkMode::Denied)?;
        println!("Zeta user runtime is already provisioned.");
        return Ok(());
    }
    win::create_private_directory(&root, &owner)?;
    let _setup_lock = setup_lock(&root)?;
    let bin = root.join("bin");
    std::fs::create_dir_all(&bin).map_err(|error| error.to_string())?;
    let runner = bin.join("zeta-windows-sandbox.exe");
    std::fs::copy(
        std::env::current_exe().map_err(|error| error.to_string())?,
        &runner,
    )
    .map_err(|error| error.to_string())?;
    let mut accounts = Vec::new();
    let mut listeners = Vec::new();
    for mode in [
        NetworkMode::Denied,
        NetworkMode::Managed,
        NetworkMode::Allowed,
    ] {
        for _ in 0..slots {
            let port = if mode == NetworkMode::Managed {
                let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
                    .map_err(|error| error.to_string())?;
                let port = listener
                    .local_addr()
                    .map_err(|error| error.to_string())?
                    .port();
                listeners.push(listener);
                port
            } else {
                0
            };
            accounts.push(account::plan(mode, port)?);
        }
    }
    let mut state = State {
        version: VERSION,
        owner,
        status: Status::Preparing,
        runner_hash: hash(&runner)?,
        device_sid: device_sid()?,
        devices: Vec::new(),
        accounts,
        rules: None,
    };
    // Persist names, passwords, and unique ownership tags before NetUserAdd.
    state.save(&root)?;
    install_devices(&mut state, &root)?;
    super::devices::verify(&state.device_sid)?;
    for index in 0..state.accounts.len() {
        account::create(&mut state.accounts[index])?;
        state.save(&root)?;
    }
    let mut sddl = format!("D:P(A;OICI;FA;;;SY)(A;OICI;FA;;;{})", state.owner);
    for account in &state.accounts {
        sddl.push_str(&format!("(A;OICI;GRGX;;;{})", account.sid));
    }
    let sd = win::descriptor(&sddl)?;
    // SetFileSecurityW does not update ACLs of existing children. The runner
    // was copied before the account identities existed, so stamp it explicitly.
    for path in [&bin, &runner] {
        if unsafe {
            SetFileSecurityW(
                win::wide(path).as_ptr(),
                DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
                sd.0,
            )
        } == 0
        {
            return Err(win::error("SetFileSecurityW(runtime binaries)"));
        }
    }
    // The service-side logon path can traverse this newly owned root without
    // changing the signed-in user's profile ACL. Account grants do not inherit
    // into credentials, leases, or future state journals.
    let mut root_acl = format!("D:P(A;OICI;FA;;;SY)(A;OICI;FA;;;{})", state.owner);
    for account in &state.accounts {
        root_acl.push_str(&format!("(A;;GRGX;;;{})", account.sid));
    }
    let root_sd = win::descriptor(&root_acl)?;
    if unsafe {
        SetFileSecurityW(
            win::wide(&root).as_ptr(),
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            root_sd.0,
        )
    } == 0
    {
        return Err(win::error("SetFileSecurityW(runtime traversal)"));
    }
    for index in 0..state.accounts.len() {
        File::create(root.join(format!("lease-{index}"))).map_err(|error| error.to_string())?;
    }
    // Every WFP GUID is journaled before the atomic network transaction begins.
    state.rules = Some(Rules::plan(&state.accounts)?);
    state.save(&root)?;
    let rules = state.rules.as_ref().unwrap();
    rules.install(&state.accounts)?;
    rules.verify(&state.accounts)?;
    state.status = Status::Ready;
    state.save(&root)?;
    println!(
        "Provisioned {} independent accounts and their scoped network rules.",
        state.accounts.len()
    );
    Ok(())
}

pub(super) fn remove(approved: &str) -> Result<()> {
    if approved != plan_digest(&removal_plan()?)? {
        return Err("removal approval does not match the recorded installation objects".into());
    }
    require_administrator()?;
    let (root, mut state) = read_state()?;
    let setup_guard = setup_lock(&root)?;
    let mut locks = Vec::new();
    for index in 0..state.accounts.len() {
        let path = root.join(format!("lease-{index}"));
        match OpenOptions::new()
            .read(true)
            .write(true)
            .share_mode(0)
            .open(path)
        {
            Ok(lock) => locks.push(lock),
            Err(error)
                if error.kind() == std::io::ErrorKind::NotFound
                    && state.status != Status::Ready => {}
            Err(_) => {
                return Err(
                    "cannot remove a runtime with active executions or unreadable leases".into(),
                );
            }
        }
    }
    state.status = Status::Removing;
    state.save(&root)?;
    for account in &state.accounts {
        super::job::Job::recover(&account.name)?;
    }
    let recovery = wxc_common::filesystem_dacl::recover_orphaned_state_in(&root.join("acl"))
        .map_err(|error| error.to_string())?;
    if !recovery.errors.is_empty() {
        return Err("ACL recovery is incomplete; keep the runtime journal for recovery".into());
    }
    for account in &state.accounts {
        let path = root.join("runs").join(&account.sid);
        if path.try_exists().map_err(|error| error.to_string())? {
            let canonical = std::fs::canonicalize(&path).map_err(|error| error.to_string())?;
            let runs =
                std::fs::canonicalize(root.join("runs")).map_err(|error| error.to_string())?;
            if canonical.parent() != Some(runs.as_path())
                || canonical.file_name() != Some(std::ffi::OsStr::new(&account.sid))
            {
                return Err("execution recovery directory was redirected".into());
            }
            std::fs::remove_dir_all(&path).map_err(|error| error.to_string())?;
        }
    }
    super::devices::remove(&state.device_sid, &state.devices)?;
    for account in &state.accounts {
        account::remove(account)?;
    }
    if let Some(rules) = &state.rules {
        rules.remove()?;
    }
    // No new execution can acquire a lease after the Removing state is saved.
    drop(locks);
    clean_files(&root, &state.runner_hash, state.accounts.len())?;
    drop(setup_guard);
    remove_file(&root.join("setup.lock"))?;
    remove_file(&root.join("state.dpapi"))?;
    std::fs::remove_dir(&root)
        .map_err(|error| format!("runtime directory remains after cleanup: {error}"))?;
    if root.try_exists().map_err(|error| error.to_string())? {
        return Err("runtime directory still exists after cleanup".into());
    }
    println!("Verified removal of the Zeta execution accounts, network rules, and runtime files.");
    Ok(())
}

fn install_devices(state: &mut State, root: &Path) -> Result<()> {
    let sid = state.device_sid.clone();
    super::devices::install(&sid, |path| {
        if !state.devices.iter().any(|value| value == path) {
            state.devices.push(path.to_owned());
        }
        state.save(root)
    })
}

fn remove_file(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("could not remove '{}': {error}", path.display())),
    }
}

fn remove_empty_directory(path: &Path) -> Result<()> {
    match std::fs::remove_dir(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!(
            "directory '{}' needs recovery or inspection: {error}",
            path.display()
        )),
    }
}

fn clean_files(root: &Path, runner_hash: &str, accounts: usize) -> Result<()> {
    use std::os::windows::fs::MetadataExt;
    use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
    // Only remove the recorded layout. Never recursively erase unknown data or
    // follow a junction, and retain the recovery journal on any mismatch.
    for entry in std::fs::read_dir(root).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            return Err("unexpected runtime entry".into());
        };
        let known = matches!(
            name,
            "state.dpapi" | "state.pending" | "setup.lock" | "bin" | "runs" | "acl"
        ) || (0..accounts).any(|index| name == format!("lease-{index}"));
        if !known
            || std::fs::symlink_metadata(entry.path())
                .map_err(|error| error.to_string())?
                .file_attributes()
                & FILE_ATTRIBUTE_REPARSE_POINT
                != 0
        {
            return Err(format!(
                "unexpected or redirected runtime entry: '{}'",
                entry.path().display()
            ));
        }
    }
    remove_empty_directory(&root.join("runs"))?;
    remove_empty_directory(&root.join("acl"))?;
    let runner = root.join("bin/zeta-windows-sandbox.exe");
    if runner.try_exists().map_err(|error| error.to_string())? {
        if std::fs::symlink_metadata(&runner)
            .map_err(|error| error.to_string())?
            .file_attributes()
            & FILE_ATTRIBUTE_REPARSE_POINT
            != 0
            || hash(&runner)? != runner_hash
        {
            return Err("the runtime executable changed; preserve it and the recovery journal for inspection".into());
        }
        remove_file(&runner)?;
    }
    remove_empty_directory(&root.join("bin"))?;
    for index in 0..accounts {
        remove_file(&root.join(format!("lease-{index}")))?;
    }
    remove_file(&root.join("state.pending"))?;
    Ok(())
}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;

fn setup_lock(root: &Path) -> Result<File> {
    OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .share_mode(0)
        .open(root.join("setup.lock"))
        .map_err(|error| error.to_string())
}

fn require_administrator() -> Result<()> {
    use windows_sys::Win32::Security::CheckTokenMembership;
    let administrators = win::sid("S-1-5-32-544")?;
    let mut member = 0;
    if unsafe { CheckTokenMembership(std::ptr::null_mut(), administrators.0, &mut member) } == 0
        || member == 0
    {
        Err("runtime provisioning requires explicit administrator approval; normal command execution does not".into())
    } else {
        Ok(())
    }
}

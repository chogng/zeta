use crate::error::hook_execution_error;
use crate::outcome::HookDecision;
use crate::outcome::parse_output;
use std::time::Duration;
use zeta_async_utils::CancellationToken;
use zeta_config::HookAction;
use zeta_config::HookConfig;
use zeta_core::CoreError;
use zeta_file_access::Dir;
use zeta_tool_executor::ApprovalPolicy;
use zeta_tool_executor::ApprovalRequirement;
use zeta_tool_executor::CommandExecutionAuthority;
use zeta_tool_executor::CommandExecutionOutcome;
use zeta_tool_executor::CommandExecutor;
use zeta_tool_executor::CommandInput;
use zeta_tool_executor::CommandRequest;
use zeta_tool_executor::ExecutionLimits;

const HOOK_TIMEOUT: Duration = Duration::from_secs(30);
const HOOK_OUTPUT_BYTES: usize = 64 * 1024;

pub(crate) trait HookProcessExecutor: Send + Sync {
    fn dir(&self) -> &Dir;

    fn execute(
        &self,
        hook: &HookConfig,
        input: Vec<u8>,
        authority: CommandExecutionAuthority,
        cancellation: &CancellationToken,
    ) -> Result<HookDecision, CoreError>;
}

struct AlwaysAuthorized;

impl ApprovalPolicy for AlwaysAuthorized {
    fn requirement_for(&self, _: &str) -> ApprovalRequirement {
        ApprovalRequirement::NotRequired
    }
}

pub(crate) struct LocalHookProcessExecutor {
    dir: Dir,
    executor: CommandExecutor<AlwaysAuthorized, PlatformSandbox>,
}

impl LocalHookProcessExecutor {
    pub(crate) fn new(dir: Dir) -> Result<Self, String> {
        let backend = platform_sandbox().map_err(|error| error.to_string())?;
        Ok(Self {
            dir: dir.clone(),
            executor: CommandExecutor::new(
                dir,
                backend,
                AlwaysAuthorized,
                ExecutionLimits {
                    timeout: HOOK_TIMEOUT,
                    max_output_bytes: HOOK_OUTPUT_BYTES,
                },
            ),
        })
    }
}

impl HookProcessExecutor for LocalHookProcessExecutor {
    fn dir(&self) -> &Dir {
        &self.dir
    }

    fn execute(
        &self,
        hook: &HookConfig,
        input: Vec<u8>,
        authority: CommandExecutionAuthority,
        cancellation: &CancellationToken,
    ) -> Result<HookDecision, CoreError> {
        let HookAction::Process { program, args } = &hook.action;
        let result = self.executor.execute(
            CommandRequest {
                program: program.clone(),
                arguments: args.clone(),
                working_directory: self.dir.canonical_path().to_path_buf(),
                input: CommandInput::Bytes(input),
            },
            authority,
            cancellation,
        );
        match result {
            Ok(CommandExecutionOutcome::Completed(output)) => {
                parse_output(hook.id.as_str(), output)
            }
            Ok(CommandExecutionOutcome::SandboxDenied(_)) => Err(CoreError::Policy(format!(
                "Hook '{}' was denied by the directory sandbox",
                hook.id
            ))),
            Err(error) => Err(CoreError::Execution(hook_execution_error(error))),
        }
    }
}

#[cfg(target_os = "macos")]
type PlatformSandbox = zeta_sandboxing::MacosSeatbeltSandbox;

#[cfg(target_os = "macos")]
fn platform_sandbox() -> Result<PlatformSandbox, String> {
    Ok(PlatformSandbox::new())
}

#[cfg(target_os = "linux")]
type PlatformSandbox = zeta_linux_sandbox::LinuxSandbox;

#[cfg(target_os = "linux")]
fn platform_sandbox() -> Result<PlatformSandbox, String> {
    PlatformSandbox::discover(&zeta_install_context::InstallContext::current())
        .map_err(|error| error.to_string())
}

#[cfg(target_os = "windows")]
type PlatformSandbox = zeta_windows_sandbox::WindowsSandbox;

#[cfg(target_os = "windows")]
fn platform_sandbox() -> Result<PlatformSandbox, String> {
    PlatformSandbox::discover(&zeta_install_context::InstallContext::current())
        .map_err(|error| error.to_string())
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
compile_error!("configured Hooks require a supported sandbox backend");

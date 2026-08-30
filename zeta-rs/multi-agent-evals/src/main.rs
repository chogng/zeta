use clap::Parser;
use clap::Subcommand;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;
use zeta_multi_agent_evals::EvalMode;
use zeta_multi_agent_evals::EvalResult;
use zeta_multi_agent_evals::EvalStatus;
use zeta_multi_agent_evals::LiveRunOptions;
use zeta_multi_agent_evals::cases;
use zeta_multi_agent_evals::find_case;
use zeta_multi_agent_evals::run_live;
use zeta_multi_agent_evals::run_scripted;

#[derive(Parser)]
#[command(name = "zeta-multi-agent-evals")]
#[command(about = "Runs versioned multi-Agent cases through Zeta and host-owned oracles")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Runs deterministic malicious model subjects without network access.
    Scripted {
        /// Runs only one stable case ID. All scripted cases run when omitted.
        #[arg(long)]
        case: Option<String>,
    },
    /// Runs a provider model configured in an explicit dedicated profile.
    Live {
        #[arg(long, default_value = "team_scope_inducement_v1")]
        case: String,
        #[arg(long)]
        profile: PathBuf,
        #[arg(long, default_value_t = 300)]
        timeout_seconds: u64,
        /// Required acknowledgement that this command can consume model tokens.
        #[arg(long)]
        acknowledge_model_cost: bool,
    },
}

fn main() -> ExitCode {
    match execute(Cli::parse()) {
        Ok(results) => {
            println!("{}", serde_json::to_string_pretty(&results).unwrap());
            if results
                .iter()
                .all(|result| result.status() == EvalStatus::Passed)
            {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(error) => {
            eprintln!("multi-Agent evaluation failed to start: {error}");
            ExitCode::from(2)
        }
    }
}

fn execute(cli: Cli) -> Result<Vec<EvalResult>, String> {
    match cli.command {
        Command::Scripted { case } => match case {
            Some(id) => Ok(vec![run_scripted(&find_case(&id, EvalMode::Scripted)?)?]),
            None => cases()?
                .into_iter()
                .filter(|case| case.modes.contains(&EvalMode::Scripted))
                .map(|case| run_scripted(&case))
                .collect(),
        },
        Command::Live {
            case,
            profile,
            timeout_seconds,
            acknowledge_model_cost,
        } => {
            if !acknowledge_model_cost {
                return Err("live mode requires --acknowledge-model-cost".into());
            }
            let case = find_case(&case, EvalMode::Live)?;
            Ok(vec![run_live(
                &case,
                &LiveRunOptions {
                    profile_root: profile,
                    timeout: Duration::from_secs(timeout_seconds),
                },
            )?])
        }
    }
}

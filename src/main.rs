use anyhow::{Context, Result};
use clap::{CommandFactory, Parser};

use zitadel_tui::{
    cli::{command_path, Cli, Command},
    commands::{self, execute_command},
    conductor::TuiConductor,
    config::AppConfig,
    error::{CommandError, ErrorKind},
    output::{CommandEnvelope, ErrorEnvelope},
    tui_runtime::run_tui_loading,
};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    if args.demo {
        if args.command.is_some() {
            Cli::command()
                .error(
                    clap::error::ErrorKind::ArgumentConflict,
                    "--demo opens the interactive TUI and cannot be used with a subcommand",
                )
                .exit();
        }
        return run_tui_loading(TuiConductor::bootstrap(args, AppConfig::default())).await;
    }

    if let Some(command) = &args.command {
        // `agent` prints the guide verbatim — a doc surface, not an API
        // command — so it bypasses the JSON envelope on purpose.
        if matches!(command, Command::Agent) {
            commands::print_agent_guide();
            return Ok(());
        }
        let path = command_path(command);
        // Config load failures are still reported in the envelope so agents
        // get a single parse target on stdout.
        let outcome = match load_config(&args) {
            Ok(config) => execute_command(command, &args, &config).await,
            Err(error) => Err(CommandError::new(ErrorKind::Config, error.to_string())),
        };
        return match outcome {
            Ok(result) => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&CommandEnvelope {
                        command: path,
                        ok: true,
                        result,
                    })?
                );
                Ok(())
            }
            Err(error) => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&ErrorEnvelope::new(path, &error))?
                );
                std::process::exit(1);
            }
        };
    }

    let config = load_config(&args)?;
    run_tui_loading(TuiConductor::bootstrap(args, config)).await
}

fn load_config(args: &Cli) -> Result<AppConfig> {
    if let Some(path) = &args.config {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let config: AppConfig = toml::from_str(&contents)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        return Ok(config);
    }

    AppConfig::load()
}

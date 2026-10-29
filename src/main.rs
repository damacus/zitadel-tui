use anyhow::{Context, Result};
use clap::Parser;

use zitadel_tui::{
    cli::{command_path, Cli},
    commands::execute_command,
    conductor::TuiConductor,
    config::AppConfig,
    error::{CommandError, ErrorKind},
    output::{CommandEnvelope, ErrorEnvelope},
    tui_runtime::run_tui,
};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    if let Some(command) = &args.command {
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
    let conductor = TuiConductor::bootstrap(args.clone(), config).await;
    run_tui(conductor).await
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

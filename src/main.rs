mod auth;
mod cli;
mod client;
mod commands;
mod conductor;
mod config;
mod oidc;
mod output;
#[cfg(test)]
mod test_support;
mod token_cache;
mod tui;
mod tui_runtime;

use anyhow::{Context, Result};
use clap::Parser;

use crate::{
    cli::{command_name, Cli},
    commands::execute_command,
    config::AppConfig,
    output::{print_human, CommandEnvelope},
    tui_runtime::run_tui,
};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    validate_cli(&args)?;
    let config = load_config(&args)?;

    if let Some(command) = &args.command {
        match execute_command(command, &args, &config).await {
            Ok(result) => {
                if args.json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&CommandEnvelope {
                            command: command_name(command),
                            ok: true,
                            result,
                        })?
                    );
                } else {
                    print_human(command, &result);
                }
                return Ok(());
            }
            Err(error) => {
                if args.json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "command": command_name(command),
                            "ok": false,
                            "error": error.to_string()
                        }))?
                    );
                    return Ok(());
                }
                return Err(error);
            }
        }
    }

    let conductor = conductor::TuiConductor::bootstrap(args.clone(), config).await;
    run_tui(conductor).await
}

fn validate_cli(args: &Cli) -> Result<()> {
    if args.once && args.command.is_none() {
        anyhow::bail!("--once is deprecated and only accepted with a subcommand");
    }

    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_command_without_once() {
        let cli = Cli::parse_from(["zitadel-tui", "apps", "list"]);
        assert!(validate_cli(&cli).is_ok());
    }

    #[test]
    fn rejects_once_without_command() {
        let cli = Cli::parse_from(["zitadel-tui", "--once"]);
        assert!(validate_cli(&cli).is_err());
    }

    #[test]
    fn accepts_once_with_command() {
        let cli = Cli::parse_from(["zitadel-tui", "--once", "apps", "list"]);
        assert!(validate_cli(&cli).is_ok());
    }

    #[test]
    fn accepts_once_with_host_and_command() {
        let cli = Cli::parse_from([
            "zitadel-tui",
            "--host",
            "https://zitadel.example.com",
            "--once",
            "apps",
            "list",
        ]);
        assert!(validate_cli(&cli).is_ok());
    }
}

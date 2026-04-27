use serde::Serialize;
use serde_json::Value;

use crate::cli::Command;

#[derive(Debug, Serialize)]
pub struct CommandEnvelope<T: Serialize> {
    pub command: String,
    pub ok: bool,
    pub result: T,
}

pub fn print_human(command: &Command, result: &Value) {
    match command {
        Command::Apps(_) | Command::Users(_) | Command::Idps(_) => {
            if let Some(items) = result.as_array() {
                for item in items {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(item).unwrap_or_else(|_| item.to_string())
                    );
                }
            } else {
                println!(
                    "{}",
                    serde_json::to_string_pretty(result).unwrap_or_else(|_| result.to_string())
                );
            }
        }
        Command::Auth(_) | Command::Config(_) => {
            println!(
                "{}",
                serde_json::to_string_pretty(result).unwrap_or_else(|_| result.to_string())
            );
        }
    }
}

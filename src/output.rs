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
    print!("{}", format_human(command, result));
}

fn format_human(command: &Command, result: &Value) -> String {
    match command {
        Command::Apps(_) | Command::Users(_) | Command::Idps(_) => {
            if let Some(items) = result.as_array() {
                if items.is_empty() {
                    format!("{}\n", empty_list_message(command))
                } else {
                    items
                        .iter()
                        .map(format_json_value)
                        .collect::<Vec<_>>()
                        .join("\n")
                        + "\n"
                }
            } else {
                format!("{}\n", format_json_value(result))
            }
        }
        Command::Auth(_) | Command::Config(_) => {
            format!("{}\n", format_json_value(result))
        }
    }
}

fn empty_list_message(command: &Command) -> &'static str {
    match command {
        Command::Apps(_) => "No apps found.",
        Command::Users(_) => "No users found.",
        Command::Idps(_) => "No identity providers found.",
        Command::Auth(_) | Command::Config(_) => {
            unreachable!("empty list messages are only for list resources")
        }
    }
}

fn format_json_value(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::cli::{
        AppsAction, AppsCommand, Command, IdpsAction, IdpsCommand, UsersAction, UsersCommand,
    };

    #[test]
    fn empty_apps_list_prints_empty_state() {
        let command = Command::Apps(AppsCommand {
            action: AppsAction::List,
        });

        assert_eq!(format_human(&command, &json!([])), "No apps found.\n");
    }

    #[test]
    fn empty_users_list_prints_empty_state() {
        let command = Command::Users(UsersCommand {
            action: UsersAction::List,
        });

        assert_eq!(format_human(&command, &json!([])), "No users found.\n");
    }

    #[test]
    fn empty_idps_list_prints_empty_state() {
        let command = Command::Idps(IdpsCommand {
            action: IdpsAction::List,
        });

        assert_eq!(
            format_human(&command, &json!([])),
            "No identity providers found.\n"
        );
    }

    #[test]
    fn non_empty_list_still_prints_pretty_json_items() {
        let command = Command::Apps(AppsCommand {
            action: AppsAction::List,
        });

        assert_eq!(
            format_human(&command, &json!([{ "id": "app-1" }])),
            "{\n  \"id\": \"app-1\"\n}\n"
        );
    }
}

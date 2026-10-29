use std::process::Command;

#[test]
fn help_flag_exits_successfully() {
    let status = Command::new(env!("CARGO_BIN_EXE_zitadel-tui"))
        .arg("--help")
        .status()
        .expect("failed to spawn zitadel-tui");

    assert!(status.success());
}

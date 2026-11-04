use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn help_displays_available_commands() {
    let mut command = Command::new(env!("CARGO_BIN_EXE_pryn"));
    command
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("diagnose"));
}

#[test]
fn diagnose_wifi_emits_json_report() {
    let mut command = Command::new(env!("CARGO_BIN_EXE_pryn"));
    let output = command
        .args(["diagnose", "wifi", "--format", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let report: serde_json::Value =
        serde_json::from_slice(&output).expect("CLI should emit valid JSON");
    assert_eq!(report["schema_version"], 1);
    assert_eq!(report["subsystems"].as_array().map(Vec::len), Some(1));
    assert_eq!(report["subsystems"][0]["subsystem"], "wifi");
}

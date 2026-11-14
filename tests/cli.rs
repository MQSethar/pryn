use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn no_arguments_show_help_with_check_command() {
    Command::new(env!("CARGO_BIN_EXE_pryn"))
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Check Wi-Fi, Bluetooth, and audio on Linux",
        ))
        .stdout(predicate::str::contains("check"))
        .stdout(predicate::str::contains("diagnose").not());
}

#[test]
fn check_json_emits_versioned_report() {
    Command::new(env!("CARGO_BIN_EXE_pryn"))
        .args(["check", "--format", "json"])
        .assert()
        .code(predicate::in_iter([0, 1]))
        .stdout(predicate::str::contains("\"schema_version\": 1"))
        .stdout(predicate::str::contains("\"subsystems\""));
}

#[test]
fn check_accepts_a_single_subsystem() {
    Command::new(env!("CARGO_BIN_EXE_pryn"))
        .args(["check", "wifi"])
        .assert()
        .code(predicate::in_iter([0, 1]))
        .stdout(predicate::str::contains("Wi-Fi"));
}

#[test]
fn old_diagnose_command_is_not_accepted() {
    Command::new(env!("CARGO_BIN_EXE_pryn"))
        .arg("diagnose")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unrecognized subcommand"));
}

#[test]
fn check_verbose_flag_is_accepted() {
    Command::new(env!("CARGO_BIN_EXE_pryn"))
        .args(["check", "wifi", "--verbose"])
        .assert()
        .code(predicate::in_iter([0, 1]))
        .stdout(predicate::str::contains("Wi-Fi"));
}

use std::{
    io::Write,
    process::{Command, Stdio},
};
fn cli(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_diagram-rs"))
        .args(args)
        .output()
        .unwrap()
}
#[test]
fn help_and_version() {
    for arg in ["--help", "--version"] {
        let o = cli(&[arg]);
        assert!(o.status.success());
        assert!(!o.stdout.is_empty());
    }
}
#[test]
fn file_and_stdin_rendering() {
    let input = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/branch.mmd"
    );
    let o = cli(&[input]);
    assert!(o.status.success());
    assert!(o.stdout.starts_with(b"<svg"));
    let mut child = Command::new(env!("CARGO_BIN_EXE_diagram-rs"))
        .args(["render", "-", "--theme", "dark"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"flowchart LR; A-->B")
        .unwrap();
    let o = child.wait_with_output().unwrap();
    assert!(o.status.success());
    assert!(String::from_utf8(o.stdout).unwrap().contains("#0f172a"));
}
#[test]
fn file_output_and_check() {
    let input = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/branch.mmd"
    );
    let path = std::env::temp_dir().join(format!("diagram-cli-test-{}.svg", std::process::id()));
    let o = cli(&[input, "-o", path.to_str().unwrap()]);
    assert!(o.status.success());
    assert!(o.stdout.is_empty());
    assert!(std::fs::read_to_string(&path).unwrap().starts_with("<svg"));
    std::fs::remove_file(path).unwrap();
    let o = cli(&["check", input]);
    assert!(o.status.success());
    assert!(
        String::from_utf8(o.stdout)
            .unwrap()
            .contains("4 nodes, 4 edges")
    );
}
#[test]
fn errors_fail_with_diagnostics() {
    for args in [
        vec!["missing.mmd"],
        vec!["render"],
        vec!["--unknown"],
        vec!["-o"],
        vec!["check", "x", "--theme", "dark"],
    ] {
        assert!(!cli(&args).status.success());
    }
    let mut child = Command::new(env!("CARGO_BIN_EXE_diagram-rs"))
        .arg("-")
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"flowchart TB\nA~~B")
        .unwrap();
    let o = child.wait_with_output().unwrap();
    assert!(!o.status.success());
    assert!(String::from_utf8(o.stderr).unwrap().contains("-:2:2:"));
}

#[test]
fn rejects_output_aliasing_input() {
    let input = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/branch.mmd"
    );
    let alias = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/../fixtures/branch.mmd"
    );
    let before = std::fs::read(input).unwrap();
    assert!(!cli(&[input, "-o", alias]).status.success());
    assert_eq!(std::fs::read(input).unwrap(), before);
}

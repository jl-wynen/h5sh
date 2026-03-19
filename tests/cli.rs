// rexpect does not work on Windows.
#![cfg(not(target_os = "windows"))]

use rexpect::session::{PtyReplSession, spawn_command};
use std::path::PathBuf;
use std::process::{Command, Stdio};

/** Return the path to a test data file. */
fn data_path(filename: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join(filename)
        .to_str()
        .unwrap()
        .to_string()
}

/** Return the path to the h5sh executable. */
fn exe_path() -> String {
    escargot::CargoBuild::new()
        .current_release()
        .current_target()
        .bin("h5sh")
        .run()
        .unwrap()
        .path()
        .to_str()
        .unwrap()
        .to_string()
}

/** Launch h5sh as an interactive PTY session. */
fn launch_h5sh() -> PtyReplSession {
    let mut cmd = Command::new(exe_path());
    cmd.arg(data_path("test.h5"))
        .arg("--color=never")
        .env("COLUMNS", "80");

    let mut h5sh = PtyReplSession::new(spawn_command(cmd, Some(200)).unwrap(), "$".to_owned())
        // h5sh echoes the input back to stdout
        .echo_on(true)
        .quit_command(Some("exit".to_owned()));
    h5sh.wait_for_prompt().unwrap();
    h5sh
}

/** Run a command in an interactive h5py session and return the output. */
fn send_command(session: &mut PtyReplSession, command: &str) -> String {
    session.send_line(command).unwrap();
    session.wait_for_prompt().unwrap();
    let input_line = session.read_line().unwrap();
    assert!(input_line.contains(command));
    session.read_line().unwrap()
}

/** Run a command that produces no output in an interactive h5py session. */
fn send_command_no_output(session: &mut PtyReplSession, command: &str) {
    session.send_line(command).unwrap();
    session.wait_for_prompt().unwrap();
    let input_line = session.read_line().unwrap();
    assert!(input_line.contains(command));
}

fn read_all_lines(session: &mut PtyReplSession) -> Vec<String> {
    let mut lines = Vec::new();
    loop {
        match session.read_line() {
            Ok(line) => lines.push(line),
            Err(rexpect::error::Error::Timeout { .. }) => break,
            Err(e) => panic!("Error reading line: {:?}", e),
        }
    }
    lines
}

/** Assert that the given output contains the expected string. */
fn assert_output_contains(actual: impl AsRef<str>, expected: &str) {
    let actual = actual.as_ref();
    assert!(
        actual.contains(expected),
        "Expected output to contain '{expected}', but got:\nvvv\n{actual}\n^^^\n"
    );
}

fn assert_output_lines(actual: Vec<impl AsRef<str>>, expected: Vec<&str>) {
    for (a, e) in actual.iter().zip(expected.iter()) {
        assert!(
            a.as_ref().starts_with(e),
            "Expected '{}' to  start with '{e}'",
            a.as_ref()
        );
    }
}

#[test]
fn cd_and_pwd() -> Result<(), Box<dyn std::error::Error>> {
    let mut h5sh = launch_h5sh();
    assert_output_contains(send_command(&mut h5sh, "pwd"), "/");
    // Can go to subgroup
    send_command_no_output(&mut h5sh, "cd base");
    assert_output_contains(send_command(&mut h5sh, "pwd"), "/base");
    // Can go up
    send_command_no_output(&mut h5sh, "cd ..");
    assert_output_contains(send_command(&mut h5sh, "pwd"), "/");
    // Can go up from root without error
    send_command_no_output(&mut h5sh, "cd ..");
    assert_output_contains(send_command(&mut h5sh, "pwd"), "/");
    // Can go to nested group
    send_command_no_output(&mut h5sh, "cd base/sub-group");
    assert_output_contains(send_command(&mut h5sh, "pwd"), "/base/sub-group");
    // Looped path gets resolved
    send_command_no_output(&mut h5sh, "cd ../sub-group/./../../base");
    assert_output_contains(send_command(&mut h5sh, "pwd"), "/base");
    Ok(())
}

#[test]
fn ls_from_root() -> Result<(), Box<dyn std::error::Error>> {
    let mut h5sh = launch_h5sh();
    assert_output_contains(send_command(&mut h5sh, "ls"), "base");

    send_command_no_output(&mut h5sh, "ls base");
    let output = read_all_lines(&mut h5sh);
    assert_eq!(
        output,
        vec!["g_empty/  label-utf8  long_array  short  sub-group/"]
    );

    Ok(())
}

#[test]
fn ls_l() -> Result<(), Box<dyn std::error::Error>> {
    let mut h5sh = launch_h5sh();
    send_command_no_output(&mut h5sh, "ls -l base");
    let output = read_all_lines(&mut h5sh);

    let expected_lines = vec![
        "            grp      g_empty/",
        "()     16B  utf-8    label-utf8 This is a UTF-8 dataset",
        "(1030)  8Ki f64      long_array [0, 1, 2, 3, 4, 5, 6, 7 ...",
        "()      6B  ascii(6) short      shorty",
        "            grp      sub-group/",
    ];
    for (actual, expected) in output.iter().zip(expected_lines.iter()) {
        assert!(
            actual.starts_with(expected),
            "Expected '{actual}' to  start with '{expected}'"
        );
    }

    Ok(())
}

#[test]
fn ls_empty_group() -> Result<(), Box<dyn std::error::Error>> {
    let mut h5sh = launch_h5sh();
    send_command_no_output(&mut h5sh, "cd base/g_empty");
    send_command_no_output(&mut h5sh, "ls");
    assert_eq!(read_all_lines(&mut h5sh), Vec::<String>::new());
    Ok(())
}

#[test]
fn attr() -> Result<(), Box<dyn std::error::Error>> {
    let mut h5sh = launch_h5sh();
    send_command_no_output(&mut h5sh, "a base/sub-group");
    let output = read_all_lines(&mut h5sh);

    let expected_lines = vec![
        "(4) 32B  i64   array@ [1, 2, 5, 6]",
        "()  16B  ascii ascii@ English only",
        "()  16B  utf-8 class@ TestGroup",
    ];
    assert_output_lines(output, expected_lines);

    Ok(())
}

#[test]
fn cat() -> Result<(), Box<dyn std::error::Error>> {
    let mut h5sh = launch_h5sh();

    send_command_no_output(&mut h5sh, "cat base/label-utf8");
    let output = read_all_lines(&mut h5sh);
    let expected_lines = vec!["This is a UTF-8 dataset"];
    assert_output_lines(output, expected_lines);

    send_command_no_output(&mut h5sh, "cat base/long_array");
    let output = read_all_lines(&mut h5sh);
    let expected_lines = vec!["[0, 1, 2, 3, 4, ..., 1025, 1026, 1027, 1028, 1029]"];
    assert_output_lines(output, expected_lines);

    Ok(())
}

#[test]
fn fd_location() -> Result<(), Box<dyn std::error::Error>> {
    let mut h5sh = launch_h5sh();

    send_command_no_output(&mut h5sh, "fd utf8");
    let output = read_all_lines(&mut h5sh);
    let expected_lines = vec!["base/label-utf8"];
    assert_output_lines(output, expected_lines);

    send_command_no_output(&mut h5sh, "fd empty");
    let output = read_all_lines(&mut h5sh);
    let expected_lines = vec!["base/g_empty"];
    assert_output_lines(output, expected_lines);

    Ok(())
}

#[test]
fn fd_attr() -> Result<(), Box<dyn std::error::Error>> {
    let mut h5sh = launch_h5sh();

    send_command_no_output(&mut h5sh, "fd @testo");
    let output = read_all_lines(&mut h5sh);
    let expected_lines = vec![
        "base/label-utf8",
        "  testo1 = test attribute 1",
        "  testo2 = another attribute",
        "base/sub-group/nested_ds",
        "  testo = nested ds",
    ];
    assert_output_lines(output, expected_lines);

    send_command_no_output(&mut h5sh, "fd @test=other");
    let output = read_all_lines(&mut h5sh);
    let expected_lines = vec!["base/label-utf8", "  testo2 = another attribute"];
    assert_output_lines(output, expected_lines);

    Ok(())
}

#[test]
fn help() -> Result<(), Box<dyn std::error::Error>> {
    let mut h5sh = launch_h5sh();
    send_command_no_output(&mut h5sh, "help");
    let output = read_all_lines(&mut h5sh);
    // `help` lists a command:
    assert!(output.iter().filter(|line| line.contains("ls")).count() > 0);
    // `help` lists an alias:
    assert!(output.iter().filter(|line| line.contains("..")).count() > 0);
    Ok(())
}

#[test]
fn file_doesnt_exist() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(exe_path())
        .arg("test/file/does_not/exist")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?
        .wait_with_output()?;

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to open file"));
    assert!(stderr.contains("No such file or directory"));

    Ok(())
}

#[test]
fn file_doesnt_exist_open_cmd() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(exe_path())
        .arg("open")
        .arg("test/file/does_not/exist")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?
        .wait_with_output()?;

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to open file"));
    assert!(stderr.contains("No such file or directory"));

    Ok(())
}

#[test]
fn self_cmd_help() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(exe_path())
        .arg("self")
        .arg("help")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?
        .wait_with_output()?;

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Manage the h5sh executable"));

    Ok(())
}

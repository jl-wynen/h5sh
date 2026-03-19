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

    let mut h5sh = PtyReplSession::new(spawn_command(cmd, Some(2000)).unwrap(), "$".to_owned())
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
    assert_eq!(output, vec!["g_empty/  label-utf8  short  sub-group/"]);

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

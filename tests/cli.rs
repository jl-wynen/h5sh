use assert_cmd::cargo::*;
use predicates::prelude::*;

#[test]
fn file_doesnt_exist() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = cargo_bin_cmd!("h5sh");
    cmd.arg("test/file/does_not/exist");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Failed to open file"))
        .stderr(predicate::str::contains("No such file or directory"));

    Ok(())
}

#[test]
fn file_doesnt_exist_open_cmd() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = cargo_bin_cmd!("h5sh");
    cmd.arg("open").arg("test/file/does_not/exist");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Failed to open file"))
        .stderr(predicate::str::contains("No such file or directory"));

    Ok(())
}

#[test]
fn file_doesnt_exist_self_cmd() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = cargo_bin_cmd!("h5sh");
    cmd.arg("self").arg("help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Manage the h5sh executable"));

    Ok(())
}


#[test]
fn list_root() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = cargo_bin_cmd!("h5sh");
    // TODO use bundled test file
    cmd.arg("data/bifrost_260301T090400.h5");

    cmd.assert()
        .success();

    // TODO interact

    Ok(())
}

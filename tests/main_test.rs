use assert_cmd::Command;

const EXECUTABLE_NAME: &str = "cli_grader";

#[test]
fn should_return_successfully() {
    let mut cmd = Command::cargo_bin(EXECUTABLE_NAME).unwrap();

    cmd.assert().success().stdout("Hello, world!\n");
}

use assert_cmd::Command;

const EXECUTABLE_NAME: &str = "clgrader";

#[test]
fn should_return_successfully() {
    let mut cmd = Command::cargo_bin(EXECUTABLE_NAME).unwrap();

    cmd.assert().success().stdout("Hello, world!, 10 - 4 = 6\n");
}

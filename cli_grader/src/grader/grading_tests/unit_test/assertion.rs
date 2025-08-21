use std::{
    io::Write,
    process::{Command, Stdio},
    thread,
};

use log::{debug, info, warn};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Assertion {
    name: String,
    // Configuration
    args: Vec<String>,
    stdin: Option<String>,
    // Expectation
    stdout: Option<String>,
    stderr: Option<String>,
    status: Option<i32>,
    // Grading
    weight: u32,
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct ExpectedObtainedResult<T> {
    expected: T,
    obtained: Option<T>,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ExecutionStatus {
    Success,
    FailureWithStatus(i32),
    FailureBeforeExecution,
    FailureBeforeWait,
    FailureWithSignalTermination,
    Undefined,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AssertionResult {
    execution_status: ExecutionStatus,
    name: String,
    passed: bool,
    weight: u32,
    stdout_diagnostics: Option<ExpectedObtainedResult<String>>,
    stderr_diagnostics: Option<ExpectedObtainedResult<String>>,
    status_diagnostics: Option<ExpectedObtainedResult<i32>>,
}

impl AssertionResult {
    fn new(name: String, weight: u32) -> Self {
        Self {
            name,
            passed: false,
            execution_status: ExecutionStatus::Undefined,
            stdout_diagnostics: None,
            stderr_diagnostics: None,
            status_diagnostics: None,
            weight,
        }
    }

    pub fn score(&self) -> u32 {
        if self.passed {
            return self.weight;
        }
        0
    }
    pub fn max_score(&self) -> u32 {
        self.weight
    }

    fn set_passed(&mut self, v: bool) {
        self.passed = v;
    }

    fn set_execution_status(&mut self, status: ExecutionStatus) {
        self.execution_status = status;
    }

    fn set_stdout_diagnostics(&mut self, expected: String, obtained: Option<String>) {
        self.stdout_diagnostics = Some(ExpectedObtainedResult { expected, obtained });
    }
    fn set_stderr_diagnostics(&mut self, expected: String, obtained: Option<String>) {
        self.stderr_diagnostics = Some(ExpectedObtainedResult { expected, obtained });
    }

    fn set_status_diagnostics(&mut self, expected: i32, obtained: Option<i32>) {
        self.status_diagnostics = Some(ExpectedObtainedResult { expected, obtained });
    }
}

impl Assertion {
    // TODO (remove) or make it test only
    pub fn new(
        name: String,
        // conf
        args: Vec<String>,
        stdin: Option<String>,
        // expect
        stdout: Option<String>,
        stderr: Option<String>,
        status: Option<i32>,
        // grading
        weight: u32,
    ) -> Self {
        Self {
            name,
            args,
            stdin,
            stdout,
            stderr,
            status,
            weight,
        }
    }

    pub fn build(
        name: String,
        // input
        args: Vec<String>,
        stdin: Option<String>,
        // expect
        stdout: Option<String>,
        stderr: Option<String>,
        status: Option<i32>,
        // grading
        weight: u32,
    ) -> Result<Self, &'static str> {
        if stdout.is_none() && stderr.is_none() && status.is_none() {
            return Err("at least one expect field must be non-null (stdout, stderr, or status)");
        }

        Ok(Self {
            name,
            args,
            stdin,
            stdout,
            stderr,
            status,
            weight,
        })
    }
    fn config_cmd(&self, cmd: &mut Command) {
        debug!("Configuring command '{:?}'", cmd.get_program());
        debug!("- Adding args: '{:?}'", self.args);
        cmd.args(&self.args)
            .stdin(if self.stdin.is_some() {
                debug!("- Setting stdin");
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(if self.stdout.is_some() {
                debug!("- Setting stdout");
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stderr(if self.stderr.is_some() {
                debug!("- Setting stderr");
                Stdio::piped()
            } else {
                Stdio::null()
            });
    }

    fn assert_stdout_stderr_status_against_null(&self, assertion_result: &mut AssertionResult) {
        if let Some(ref expected_stdout) = self.stdout {
            assertion_result.set_stdout_diagnostics(expected_stdout.clone(), None);
        }
        if let Some(ref expected_stderr) = self.stderr {
            assertion_result.set_stderr_diagnostics(expected_stderr.clone(), None);
        }
        if let Some(expected_status) = self.status {
            assertion_result.set_status_diagnostics(expected_status, None);
        }
    }

    pub fn unsafe_assert_cmd(&self, mut cmd: Command) -> AssertionResult {
        info!("🚀 Executing assertion: '{}'", self.name);
        warn!("⚠️  This assertion is UNSAFE!");
        self.config_cmd(&mut cmd);

        let mut assertion_result = AssertionResult::new(self.name.clone(), self.weight);
        info!("🔄 Trying to execute the program...");
        let mut child = match cmd.spawn() {
            Ok(handler) => handler,
            Err(err) => {
                warn!("❌ Unable to execute the command");
                debug!("💥 Error: '{err:?}'");
                info!("❌ Assertion not passed");
                assertion_result.set_execution_status(ExecutionStatus::FailureBeforeExecution);
                self.assert_stdout_stderr_status_against_null(&mut assertion_result);
                return assertion_result;
            }
        };

        if let Some(ref stdin_content) = self.stdin {
            info!("📥 Injecting stdin");
            debug!("📝 stdin: '{}'", stdin_content.replace('\n', "\\n"));
            let mut stdin = child
                .stdin
                .take()
                .expect("expected stdin from configuration");
            let stdin_content = stdin_content.clone();

            thread::spawn(move || stdin.write_all(stdin_content.as_bytes()));
        }

        info!("Trying to wait the command to finish");
        let output = match child.wait_with_output() {
            Ok(output) => output,
            Err(err) => {
                warn!("⏱️  Unable to wait the command finish");
                debug!("💥 Error: '{err:?}'");
                info!("❌ Assertion not passed");
                assertion_result.set_execution_status(ExecutionStatus::FailureBeforeWait);
                self.assert_stdout_stderr_status_against_null(&mut assertion_result);
                return assertion_result;
            }
        };
        if !output.stdout.is_empty() {
            debug!(
                "- STDOUT: '{}'",
                String::from_utf8_lossy(&output.stdout).replace('\n', "\\n")
            );
        }
        if !output.stderr.is_empty() {
            debug!(
                "- STDERR: '{}'",
                String::from_utf8_lossy(&output.stdout).replace('\n', "\\n")
            );
        }
        debug!("Output details: {output:?}");

        let mut passed = true;
        if output.status.success() {
            if let Some(expected_status) = self.status
                && expected_status != 0
            {
                debug!("  ❌ Failed status assertion.");
                debug!("   -📋 Expected: {expected_status}");
                debug!("   -📊 Obtained: 0 (success)");
                passed = false;
                assertion_result.set_status_diagnostics(expected_status, Some(0));
            }
            assertion_result.set_execution_status(ExecutionStatus::Success);
        } else {
            match output.status.code() {
                Some(obtained_status) => {
                    if let Some(expected_status) = self.status
                        && expected_status != obtained_status
                    {
                        debug!("  ❌ Failed status assertion.");
                        debug!("   -📋 Expected: {expected_status}");
                        debug!("   -📊 Obtained: {obtained_status}");
                        passed = false;
                        assertion_result
                            .set_status_diagnostics(expected_status, Some(obtained_status));
                    }
                    assertion_result
                        .set_execution_status(ExecutionStatus::FailureWithStatus(obtained_status))
                }
                None => {
                    if let Some(expected_status) = self.status {
                        debug!("  ❌ Failed status assertion.");
                        debug!("   -📋 Expected: {expected_status}");
                        debug!("   -📊 Obtained: None");
                        passed = false;
                        assertion_result.set_status_diagnostics(expected_status, None);
                    }
                    assertion_result
                        .set_execution_status(ExecutionStatus::FailureWithSignalTermination);
                }
            }
        }

        if let Some(ref expected_stdout) = self.stdout
            && output.stdout != expected_stdout.as_bytes()
        {
            debug!("  ❌ Failed stdout assertion.");
            debug!(
                "   -📋 Expected: '{}'",
                expected_stdout.replace('\n', "\\n")
            );
            debug!(
                "   -📊 Obtained: '{}'",
                String::from_utf8_lossy(&output.stdout).replace('\n', "\\n")
            );
            passed = false;
            assertion_result.set_stdout_diagnostics(
                expected_stdout.clone(),
                Some(String::from_utf8_lossy(&output.stdout).into_owned()),
            );
        }
        if let Some(ref expected_stderr) = self.stderr
            && output.stderr != expected_stderr.as_bytes()
        {
            debug!("  ❌ Failed stderr assertion.");
            debug!(
                "   -📋 Expected: '{}'",
                expected_stderr.replace('\n', "\\n")
            );
            debug!(
                "   -📊 Obtained: '{}'",
                String::from_utf8_lossy(&output.stderr).replace('\n', "\\n")
            );
            passed = false;
            assertion_result.set_stderr_diagnostics(
                expected_stderr.clone(),
                Some(String::from_utf8_lossy(&output.stderr).into_owned()),
            );
        }

        assertion_result.set_passed(passed);
        if passed {
            info!("✅ Assertion passed");
        } else {
            info!("❌ Assertion not passed");
        }
        info!("----------------------------------------------------------");
        assertion_result
    }

    /// Returns the expected result of the assertion `self` applied to some program
    /// adjusted by its fields and the parameters of this function.
    /// This is useful for testing purposes, avoiding manual building the assertion result.
    ///
    /// If `passed`, the assertion will be constructed without diagnostics. Otherwise, the
    /// diagnostics must be defined by passing values to `obtained_stdout`,
    /// `obtained_stderr`, and `obtained_status`.
    ///
    /// # Parameters
    /// - `execution_status_if_no_status`: this parameter is mandatory when no status is
    /// expected. In that situation, this execution status will be used as the expected one.
    /// - `passed`: specify if it is expected to pass the assertion or not.
    /// - `obtained_stdout`: specify the obtained stdout. This helps complementing the
    /// stdout diagnostics.
    /// - `obtained_stderr`: specify the obtained stderr. This helps complementing the
    /// stderr diagnostics.
    /// - `obtained_status`: specify the obtained status. This helps complementing the
    /// status diagnostics.
    #[cfg(test)]
    pub fn expected_result(
        &self,
        execution_status_if_no_status: Option<ExecutionStatus>,
        passed: bool,
        obtained_stdout: Option<String>,
        obtained_stderr: Option<String>,
        obtained_status: Option<i32>,
    ) -> AssertionResult {
        let execution_status = if let Some(status) = self.status {
            if status == 0 {
                ExecutionStatus::Success
            } else {
                ExecutionStatus::FailureWithStatus(status)
            }
        } else {
            execution_status_if_no_status
                .expect("no status was defined, thus, execution status must be defined manually")
        };
        let stdout_diagnostics = if let Some(stdout) = self.stdout.clone() {
            if passed {
                None
            } else {
                Some(ExpectedObtainedResult {
                    expected: stdout,
                    obtained: obtained_stdout,
                })
            }
        } else {
            None
        };
        let stderr_diagnostics = if let Some(stderr) = self.stderr.clone() {
            if passed {
                None
            } else {
                Some(ExpectedObtainedResult {
                    expected: stderr,
                    obtained: obtained_stderr,
                })
            }
        } else {
            None
        };
        let status_diagnostics = if let Some(status) = self.status {
            if passed {
                None
            } else {
                Some(ExpectedObtainedResult {
                    expected: status,
                    obtained: obtained_status,
                })
            }
        } else {
            None
        };
        AssertionResult {
            execution_status,
            name: self.name.clone(),
            passed,
            weight: self.weight,
            stdout_diagnostics,
            stderr_diagnostics,
            status_diagnostics,
        }
    }

    #[cfg(test)]
    pub fn new_dummy(
        id: u32,
        with_stdin: bool,
        with_stdout: bool,
        with_stderr: bool,
        status: Option<i32>,
        weight: u32,
    ) -> Self {
        Self::new(
            format!("name {id}"),
            (0..4)
                .into_iter()
                .map(|i| format!("arg {}", i + id))
                .collect(),
            if with_stdin {
                Some(format!("expected stdin: {id}"))
            } else {
                None
            },
            if with_stdout {
                Some(format!("expected stdout: {id}"))
            } else {
                None
            },
            if with_stderr {
                Some(format!("expected stderr: {id}"))
            } else {
                None
            },
            status,
            weight,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    //use test_log;

    mod unsecure_assert_cmd_test {
        use super::*;
        #[test]
        fn should_expect_failure_before_execution() {
            let args = vec!["arg1".to_string(), "arg2".to_string(), "arg3".to_string()];
            let expected_stdout = Some("stdout 1".to_string());
            let expected_stderr = Some("stderr 1".to_string());
            let expected_status = Some(0);
            let assertion_name = "name 123".to_string();
            let assertion_weight = 1;
            let not_passed_assertion = Assertion::new(
                assertion_name.clone(),
                args,
                Some("stdin 1".to_string()),
                expected_stdout.clone(),
                expected_stderr.clone(),
                expected_status,
                assertion_weight,
            );

            let mut cmd = Command::new("____invalid_command");

            cmd.env_clear();

            let result = not_passed_assertion.unsafe_assert_cmd(cmd);
            assert_eq!(
                result.execution_status,
                ExecutionStatus::FailureBeforeExecution
            );
            assert_eq!(result.name, assertion_name);
            assert_eq!(result.passed, false, "assertion should not pass");
            assert_eq!(result.weight, assertion_weight);
            let stdout_diagnostics = result.stdout_diagnostics;
            assert!(stdout_diagnostics.is_some_and(|v| {
                if v.expected == expected_stdout.unwrap() && v.obtained.is_none() {
                    return true;
                }
                false
            }));

            let stderr_diagnostics = result.stderr_diagnostics;
            assert!(stderr_diagnostics.is_some_and(|v| {
                if v.expected == expected_stderr.unwrap() && v.obtained.is_none() {
                    return true;
                }
                false
            }));
            let status_diagnostics = result.status_diagnostics;
            assert!(status_diagnostics.is_some_and(|v| {
                if v.expected == expected_status.unwrap() && v.obtained.is_none() {
                    return true;
                }
                false
            }));
        }

        #[test]
        fn should_expect_success_for_echo() {
            let args = vec![
                "arg1".to_string(),
                "arg2  0".to_string(),
                "arg3".to_string(),
            ];

            // Passing expectation
            let passing_expected_stdout = Some("arg1 arg2  0 arg3\n".to_string());
            let passing_expected_stderr = Some("".to_string());
            let passing_expected_status = Some(0);

            let assertion_name = "assertion name".to_string();
            let assertion_weight = 3;
            let passed_assertion = Assertion::new(
                assertion_name.clone(),
                args.clone(),
                None,
                passing_expected_stdout.clone(),
                passing_expected_stderr.clone(),
                passing_expected_status,
                assertion_weight,
            );

            let cmd = Command::new("echo");

            let result = passed_assertion.unsafe_assert_cmd(cmd);
            assert_eq!(
                result,
                AssertionResult {
                    execution_status: ExecutionStatus::Success,
                    name: assertion_name.clone(),
                    passed: true,
                    weight: assertion_weight,
                    stdout_diagnostics: None,
                    stderr_diagnostics: None,
                    status_diagnostics: None
                }
            );

            // Not passing expectation
            let not_passing_expected_stdout = Some("arg1 arg2 0 arg3".to_string());
            let not_passing_expected_stderr = Some("invalid error".to_string());
            let not_passing_expected_status = Some(23);

            let not_passed_assertion = Assertion::new(
                assertion_name.clone(),
                args,
                None,
                not_passing_expected_stdout.clone(),
                not_passing_expected_stderr.clone(),
                not_passing_expected_status,
                assertion_weight,
            );

            let cmd = Command::new("echo");

            let result = not_passed_assertion.unsafe_assert_cmd(cmd);
            assert_eq!(
                result,
                AssertionResult {
                    execution_status: ExecutionStatus::Success,
                    name: assertion_name,
                    passed: false,
                    weight: assertion_weight,
                    stdout_diagnostics: Some(ExpectedObtainedResult {
                        expected: not_passing_expected_stdout.unwrap(),
                        obtained: passing_expected_stdout
                    }),
                    stderr_diagnostics: Some(ExpectedObtainedResult {
                        expected: not_passing_expected_stderr.unwrap(),
                        obtained: passing_expected_stderr
                    }),
                    status_diagnostics: Some(ExpectedObtainedResult {
                        expected: not_passing_expected_status.unwrap(),
                        obtained: passing_expected_status
                    })
                }
            );
        }
        #[test]
        fn should_expect_success_for_cat_using_stdin() {
            // Passing expectation
            let stdin = Some("this is the input    !\n and this also".to_string());
            let passing_expected_stdout =
                Some("this is the input    !\n and this also".to_string());
            let passing_expected_stderr = Some("".to_string());
            let passing_expected_status = Some(0);

            let assertion_name = "assertion name".to_string();
            let assertion_weight = 8;
            let passed_assertion = Assertion::new(
                assertion_name.clone(),
                vec![],
                stdin.clone(),
                passing_expected_stdout.clone(),
                passing_expected_stderr.clone(),
                passing_expected_status,
                assertion_weight,
            );

            let cmd = Command::new("cat");

            let result = passed_assertion.unsafe_assert_cmd(cmd);

            assert_eq!(
                result,
                AssertionResult {
                    execution_status: ExecutionStatus::Success,
                    name: assertion_name.clone(),
                    passed: true,
                    weight: assertion_weight,
                    stdout_diagnostics: None,
                    stderr_diagnostics: None,
                    status_diagnostics: None
                }
            );

            // Not passing expectation
            // (one space missing)
            let not_passing_expected_stdout =
                Some("this is the input   !\n and this also".to_string());

            let not_passed_assertion = Assertion::new(
                assertion_name.clone(),
                vec![],
                stdin.clone(),
                not_passing_expected_stdout.clone(),
                passing_expected_stderr.clone(),
                passing_expected_status,
                assertion_weight,
            );

            let cmd = Command::new("cat");

            let result = not_passed_assertion.unsafe_assert_cmd(cmd);
            assert_eq!(
                result,
                AssertionResult {
                    execution_status: ExecutionStatus::Success,
                    name: assertion_name,
                    passed: false,
                    weight: assertion_weight,
                    stdout_diagnostics: Some(ExpectedObtainedResult {
                        expected: not_passing_expected_stdout.unwrap(),
                        obtained: passing_expected_stdout
                    }),
                    stderr_diagnostics: None,
                    status_diagnostics: None
                }
            );
        }
    }

    mod config_cmd_test {
        use super::*;
        use std::ffi::OsString;

        #[test]
        fn should_configure_every_field() {
            let expected_args = vec!["arg1".to_string(), "arg2".to_string()];
            let expected_stdout = Some("stdout 1".to_string());
            let expected_stderr = Some("stderr 1".to_string());
            let expected_status = Some(13);
            let a = Assertion {
                name: "name 1".to_string().clone(),
                args: expected_args.clone(),
                stdin: Some("stdin 1".to_string()).clone(),
                stdout: expected_stdout.clone(),
                stderr: expected_stderr.clone(),
                status: expected_status,
                weight: 1,
            };
            let mut cmd = Command::new("some command");
            a.config_cmd(&mut cmd);

            assert_eq!(
                cmd.get_args().collect::<Vec<_>>(),
                expected_args
                    .iter()
                    .map(|s| OsString::from(s))
                    .collect::<Vec<_>>()
            );
        }
    }
}

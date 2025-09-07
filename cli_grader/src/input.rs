//! This module provides the interface between the target artifact to be assessed and the
//! cli_grader.
//!
//! Everything related to how to execute some artifact or if the artifact is even valid,
//! will be implemented here.

use is_executable::is_executable;
use std::{fmt::Debug, path::PathBuf, process::Command};

/// This is the common interface to represent anything that may is executable and thus
/// testable by this framework. The executable is able to generate a
/// `std::process::Command` which effectively may be executed.
///
/// # Caveats
/// - It is important not to confuse `ExecutableArtifact` with an executable in the
///   traditional meaning (i.e. binary executable). A binary executable may be an
///   `ExecutableArtifact`, but not necessarily the latter will be the former. An
///   `ExecutableArtifact` may also appear in other formats, like Python source code or any
///   other programming languages' source code.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum ExecutableArtifact {
    CompiledProgram { name: String, path: PathBuf },
    // PythonProgram
    // JavascriptProgram
}

pub enum ProgramType {
    Compiled,
}

impl ExecutableArtifact {
    pub fn build(
        name: String,
        path: PathBuf,
        program_type: ProgramType,
    ) -> Result<Self, &'static str> {
        match program_type {
            ProgramType::Compiled => {
                // try to read it as file
                if !is_executable(&path) {
                    return Err("path does not point to an executable");
                }

                Ok(ExecutableArtifact::CompiledProgram { name, path })
            }
        }
    }

    pub fn new_cmd(&self) -> Command {
        match self {
            ExecutableArtifact::CompiledProgram { path, .. } => Command::new(path),
        }
    }

    pub fn name(&self) -> String {
        match self {
            ExecutableArtifact::CompiledProgram { name, .. } => name.to_string(),
        }
    }

    #[cfg(test)]
    pub fn new_dummy(n: usize) -> Self {
        use tempfile::NamedTempFile;

        let path = NamedTempFile::new().unwrap();
        let path = path.path();
        Self::CompiledProgram {
            name: format!("program{n}"),
            path: PathBuf::from(path),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils;
    use std::str::FromStr;

    #[test]
    fn should_build_a_valid_executable() {
        let path = utils::create_dummy_executable();
        ExecutableArtifact::build("some name".to_string(), path, ProgramType::Compiled).unwrap();
    }

    #[test]
    #[should_panic]
    fn should_fail_to_build_an_invalid_executable() {
        ExecutableArtifact::build(
            "some name".to_string(),
            PathBuf::from_str("invalid_path").unwrap(),
            ProgramType::Compiled,
        )
        .unwrap();
    }
}

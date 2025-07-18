// TODO (enhance): for now, a command will be represented as a string. Maybe, that can be
// changed to something more robust.
// TODO (improve): maybe there is a better way to represent filesystem paths
// other than `String`.

use std::{ffi::OsStr, fmt::Debug, path::Path, process::Command};

// Input
// TODO (refactor): check the necessity to create a trait here.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct CompiledProgram<'a> {
    name: String,
    path: &'a Path,
}

impl<'a> CompiledProgram<'a> {
    pub fn new<S: AsRef<OsStr> + ?Sized>(name: String, path: &'a S) -> Self {
        Self {
            name,
            path: Path::new(path),
        }
    }
    pub fn get_path(&self) -> &Path {
        &self.path
    }
}

impl<'a> Executable for CompiledProgram<'a> {
    fn new_cmd(&self) -> Command {
        Command::new(&self.path)
    }

    fn description(&self) -> String {
        self.name.to_string()
    }
}

pub trait Executable: Debug {
    fn new_cmd(&self) -> Command;
    fn description(&self) -> String;
}

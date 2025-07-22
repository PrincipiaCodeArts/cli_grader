// TODO (enhance): for now, a command will be represented as a string. Maybe, that can be
// changed to something more robust.
// TODO (improve): maybe there is a better way to represent filesystem paths
// other than `String`.
pub type Path = String;

// Input
// TODO (refactor): check the necessity to create a trait here.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct TargetProgram {
    name: String,
    path: Path,
}

impl TargetProgram {
    pub fn new(name: String, path: Path) -> Self {
        Self { name, path }
    }
    pub fn get_path(&self) -> &Path {
        &self.path
    }
}

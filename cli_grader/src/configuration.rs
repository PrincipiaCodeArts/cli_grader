use std::collections::{HashMap, HashSet};

// TODO (optimization opportunity): replace `String` with &'a str for the string
// fields.

// Grading
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum GradingMode {
    Absolute,
    Weighted,
}
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct GradingConfig {
    mode: GradingMode,
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct UnitTest {}

#[derive(Debug, PartialEq, Eq, Clone)]
enum Tests {
    UnitTest {
        env: HashSet<String>,
        setup: Vec<String>,
        teardown: Vec<String>,
        unit_tests: Vec<HashMap<TargetProgram, UnitTest>>,
    },
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct Section {
    name: Option<String>,
    weight: u32,
    tests: Tests,
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct GradingSections {
    sections: Vec<Section>,
}

// Report
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
enum ReportOutput {
    Txt,
    Stdout,
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct ReportConfig {
    is_verbose: bool,
    output: HashSet<ReportOutput>,
}

// Input
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
struct TargetProgram {
    name: String,
    // TODO (improve): maybe there is a better way to represent filesystem paths
    // other than `String`.
    path: String,
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct InputConfig {
    target_program: TargetProgram,
}

// Generic
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum LoggingMode {
    Silent,
    Normal,
    Verbose,
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct Configuration {
    name: String,
    author: String,
    logging_mode: LoggingMode,
    report_config: ReportConfig,
    input_config: InputConfig,

    grading_config: GradingConfig,
    grading_sections: GradingSections,
}

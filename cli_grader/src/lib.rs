#[allow(dead_code)]
mod grader;

#[allow(dead_code)]
mod config;
#[allow(dead_code)]
mod input;
mod report;

pub use grader::Grader;
pub use grader::GradingConfig;
pub use grader::GradingResult;
pub use grader::score::GradingMode;
use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize, Debug, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
enum LoggingMode {
    Silent,
    #[default]
    Normal,
    Verbose,
}

// ignore below
pub fn add(left: u64, right: u64) -> u64 {
    // use grader
    let conf = GradingConfig::new(
        "Test".to_string(),
        Some("test author".to_string()),
        GradingMode::Weighted,
    );
    let grader = Grader::new(&conf);
    grader.run();
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}

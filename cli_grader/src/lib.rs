/*
mod configuration;
mod input;
mod report;
*/

#[allow(dead_code)]
mod grader;

pub use grader::Grader;
pub use grader::GradingConfig;
pub use grader::GradingResult;
pub use grader::score::Mode;

// ignore below
pub fn add(left: u64, right: u64) -> u64 {
    // use grader
    let conf = GradingConfig::new(
        "Test".to_string(),
        "test author".to_string(),
        Mode::Weighted,
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

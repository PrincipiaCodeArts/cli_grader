use std::ops::{AddAssign, Mul};

/// The way that the score will be calculated.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Mode {
    /// Score will be binary (0 or 1, true or false).
    Absolute,
    /// Score will range from 0 to the total of weight.
    Weighted,
}

/// The actual score. It mirrors the structure of `Mode`.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Score {
    Absolute(bool),
    Weighted { current: u32, max: u32 },
}

impl Score {
    /// Creates a default version for `Score` which represents the 0 in the chosen mode.
    pub fn default(grading_mode: Mode) -> Self {
        match grading_mode {
            Mode::Absolute => Self::Absolute(false),
            Mode::Weighted => Self::Weighted { current: 0, max: 0 },
        }
    }
}

impl AddAssign for Score {
    fn add_assign(&mut self, rhs: Self) {
        match (self, rhs) {
            (Score::Absolute(b1), Score::Absolute(b2)) => *b1 = *b1 && b2,
            (
                Score::Weighted {
                    current: c1,
                    max: m1,
                },
                Score::Weighted {
                    current: c2,
                    max: m2,
                },
            ) => {
                *c1 = *c1 + c2;
                *m1 = *m1 + m2;
            }
            _ => panic!("unexpected addition between different scoring modes"),
        };
    }
}

impl Mul<u32> for Score {
    type Output = Score;

    fn mul(self, rhs: u32) -> Self::Output {
        match self {
            Score::Weighted { current: c, max: m } => Score::Weighted {
                current: c * rhs,
                max: m * rhs,
            },
            Score::Absolute(b) => Score::Absolute(b),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod mul_tests {
        use super::*;

        #[test]
        fn should_multiply_score_correctly() {
            // Absolute mode
            assert_eq!(Score::Absolute(false) * 23, Score::Absolute(false));
            assert_eq!(Score::Absolute(true) * 3, Score::Absolute(true));

            // Weighted mode
            assert_eq!(
                Score::Weighted {
                    current: 1,
                    max: 10
                } * 8,
                Score::Weighted {
                    current: 8,
                    max: 80
                }
            );
            assert_eq!(
                Score::Weighted {
                    current: 1,
                    max: 10
                } * 0,
                Score::Weighted { current: 0, max: 0 }
            );
        }
    }
    mod add_assign_tests {
        use super::*;

        #[test]
        #[should_panic]
        fn should_panic_when_adding_incompatible_modes() {
            let mut score = Score::default(Mode::Weighted);
            score += Score::Absolute(true);
        }

        #[test]
        fn should_add_assign_score_correctly() {
            // Absolute mode
            let mut score = Score::Absolute(false);
            score += Score::Absolute(false);
            assert_eq!(score, Score::Absolute(false));
            score += Score::Absolute(true);
            assert_eq!(score, Score::Absolute(false));

            // Weighted mode
            let mut score = Score::Weighted {
                current: 0,
                max: 10,
            };
            score += Score::Weighted { current: 0, max: 2 };
            assert_eq!(
                score,
                Score::Weighted {
                    current: 0,
                    max: 12
                }
            );
            score += Score::Weighted {
                current: 23,
                max: 25,
            };
            assert_eq!(
                score,
                Score::Weighted {
                    current: 23,
                    max: 37
                }
            );
        }
    }
}

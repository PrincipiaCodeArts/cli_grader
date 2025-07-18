use std::ops::{Add, AddAssign, Mul};

//TODO add the trait for scorable

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum GradingMode {
    Absolute,
    Weighted,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Score {
    AbsoluteScore(bool),
    WeightedScore { current: u32, max: u32 },
}

impl Score {
    pub fn default(grading_mode: GradingMode) -> Self {
        match grading_mode {
            GradingMode::Absolute => Self::AbsoluteScore(false),
            GradingMode::Weighted => Self::WeightedScore { current: 0, max: 0 },
        }
    }
}

impl AddAssign for Score {
    fn add_assign(&mut self, rhs: Self) {
        match (self, rhs) {
            (Score::AbsoluteScore(b1), Score::AbsoluteScore(b2)) => *b1 = *b1 && b2,
            (
                Score::WeightedScore {
                    current: c1,
                    max: m1,
                },
                Score::WeightedScore {
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
            Score::WeightedScore { current: c, max: m } => Score::WeightedScore {
                current: c * rhs,
                max: m * rhs,
            },
            Score::AbsoluteScore(b) => Score::AbsoluteScore(b),
        }
    }
}

impl Add for Score {
    type Output = Score;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Score::AbsoluteScore(b1), Score::AbsoluteScore(b2)) => Self::AbsoluteScore(b1 && b2),
            (
                Score::WeightedScore {
                    current: c1,
                    max: m1,
                },
                Score::WeightedScore {
                    current: c2,
                    max: m2,
                },
            ) => Self::WeightedScore {
                current: c1 + c2,
                max: m1 + m2,
            },
            _ => panic!("unexpected addition between different scoring modes"),
        }
    }
}

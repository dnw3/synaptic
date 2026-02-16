#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalCase {
    pub expected: String,
    pub actual: String,
}

impl EvalCase {
    pub fn new(expected: impl Into<String>, actual: impl Into<String>) -> Self {
        Self {
            expected: expected.into(),
            actual: actual.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvalReport {
    pub total: usize,
    pub passed: usize,
    pub accuracy: f32,
}

impl EvalReport {
    pub fn from_cases(cases: Vec<EvalCase>) -> Self {
        let total = cases.len();
        let passed = cases
            .iter()
            .filter(|case| case.expected == case.actual)
            .count();
        let accuracy = if total == 0 {
            0.0
        } else {
            passed as f32 / total as f32
        };
        Self {
            total,
            passed,
            accuracy,
        }
    }
}

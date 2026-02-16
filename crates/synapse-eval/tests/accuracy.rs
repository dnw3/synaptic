use synapse_eval::{EvalCase, EvalReport};

#[test]
fn computes_accuracy() {
    let report = EvalReport::from_cases(vec![
        EvalCase::new("a", "a"),
        EvalCase::new("b", "x"),
        EvalCase::new("c", "c"),
    ]);

    assert_eq!(report.total, 3);
    assert_eq!(report.passed, 2);
    assert!((report.accuracy - 0.666_666_7).abs() < 0.000_1);
}

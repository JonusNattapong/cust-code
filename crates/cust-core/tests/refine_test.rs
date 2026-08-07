use cust_core::{HistoryItem, RefineEngine};

#[test]
fn test_refine_engine() {
    let mut engine = RefineEngine::new();
    let history = vec![HistoryItem::User("Refine trajectory test".to_string())];

    let summary = engine.refine_trajectory(&history);
    assert!(summary.contains("Observed trajectory"));
    assert_eq!(engine.memories().len(), 1);
}

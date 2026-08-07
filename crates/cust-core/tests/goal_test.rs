use cust_core::{GoalStatus, GoalTracker, HeartbeatScheduler};

#[test]
fn test_goal_tracker() {
    let mut tracker = GoalTracker::new();
    tracker.set_goal("g1", "Implement Prime Agent goal tracking");

    assert_eq!(tracker.active_goals().len(), 1);

    tracker.pass_gate("g1", "cargo_test_passed");
    assert!(matches!(
        tracker.active_goals()[0].status,
        GoalStatus::PassedGate(_)
    ));

    tracker.complete_goal("g1");
    assert_eq!(tracker.active_goals().len(), 0);
}

#[test]
fn test_heartbeat_scheduler() {
    let mut scheduler = HeartbeatScheduler::new(60);
    assert!(scheduler.should_tick(100));
    assert!(!scheduler.should_tick(120));
    assert!(scheduler.should_tick(161));
}

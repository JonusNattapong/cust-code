use cust_core::{ReminderKind, ReminderRegistry};

#[test]
fn test_reminder_registry_render() {
    let mut reg = ReminderRegistry::new();
    reg.add(ReminderKind::TokenBudget {
        tokens_left: 15000,
        context_window: 128000,
    });

    let block = reg.render_system_reminder_block();
    assert!(block.contains("System Reminders"));
    assert!(block.contains("15000 / 128000"));
}

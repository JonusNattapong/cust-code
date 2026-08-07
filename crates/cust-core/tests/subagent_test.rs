use cust_core::{MAX_SUBAGENT_DEPTH, SubagentManager, TaskWaitMode};

#[test]
fn test_subagent_spawn_and_output() {
    let mgr = SubagentManager::new();

    let task_id = mgr.spawn_task(0, "Run background check").unwrap();
    assert!(task_id.contains("subagent-task"));

    let output = mgr.get_output(&task_id).unwrap();
    assert!(output.contains("Run background check"));

    let outputs = mgr.wait_tasks(&[task_id.clone()], TaskWaitMode::All);
    assert_eq!(outputs.len(), 1);

    assert!(mgr.kill_task(&task_id));
}

#[test]
fn test_subagent_depth_limit() {
    let mgr = SubagentManager::new();
    let res = mgr.spawn_task(MAX_SUBAGENT_DEPTH, "Too deep");
    assert!(res.is_err());
    assert!(
        res.unwrap_err()
            .to_string()
            .contains("depth limit exceeded")
    );
}

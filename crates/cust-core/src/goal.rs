use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GoalStatus {
    InProgress,
    PassedGate(String),
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub id: String,
    pub objective: String,
    pub status: GoalStatus,
    pub progress_notes: Vec<String>,
}

#[derive(Default)]
pub struct GoalTracker {
    goals: Vec<Goal>,
}

impl GoalTracker {
    pub fn new() -> Self {
        Self { goals: Vec::new() }
    }

    pub fn set_goal(&mut self, id: &str, objective: &str) {
        self.goals.push(Goal {
            id: id.to_string(),
            objective: objective.to_string(),
            status: GoalStatus::InProgress,
            progress_notes: Vec::new(),
        });
    }

    pub fn pass_gate(&mut self, id: &str, gate_name: &str) {
        if let Some(g) = self.goals.iter_mut().find(|g| g.id == id) {
            g.status = GoalStatus::PassedGate(gate_name.to_string());
            g.progress_notes.push(format!("Passed gate: {gate_name}"));
        }
    }

    pub fn complete_goal(&mut self, id: &str) {
        if let Some(g) = self.goals.iter_mut().find(|g| g.id == id) {
            g.status = GoalStatus::Completed;
            g.progress_notes.push("Goal completed.".to_string());
        }
    }

    pub fn active_goals(&self) -> Vec<&Goal> {
        self.goals
            .iter()
            .filter(|g| g.status != GoalStatus::Completed)
            .collect()
    }
}

pub struct HeartbeatScheduler {
    interval_secs: u64,
    last_tick: u64,
}

impl HeartbeatScheduler {
    pub fn new(interval_secs: u64) -> Self {
        Self {
            interval_secs,
            last_tick: 0,
        }
    }

    pub fn should_tick(&mut self, now_secs: u64) -> bool {
        if self.last_tick == 0 || now_secs >= self.last_tick + self.interval_secs {
            self.last_tick = now_secs;
            true
        } else {
            false
        }
    }
}

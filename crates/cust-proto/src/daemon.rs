use crate::acp::{AcpRequest, AcpResponse};
use crate::idempotency::IdempotencyJournal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonStatus {
    pub active_clients: usize,
    pub active_workers: usize,
    pub pid: u32,
}

pub struct DaemonSupervisor {
    journal: IdempotencyJournal,
}

impl Default for DaemonSupervisor {
    fn default() -> Self {
        Self::new()
    }
}

impl DaemonSupervisor {
    pub fn new() -> Self {
        Self {
            journal: IdempotencyJournal::new(),
        }
    }

    pub fn status(&self) -> DaemonStatus {
        DaemonStatus {
            active_clients: 1,
            active_workers: 1,
            pid: std::process::id(),
        }
    }

    pub fn handle_request(&mut self, client_id: &str, req: AcpRequest) -> AcpResponse {
        let cmd_id = req.id.to_string();
        if let Err(err) = self.journal.record_before_dispatch(client_id, &cmd_id) {
            return AcpResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: None,
                error: Some(crate::acp::AcpError {
                    code: -32000,
                    message: err.to_string(),
                }),
            };
        }

        let res = serde_json::json!({ "status": "acknowledged", "method": req.method });
        self.journal.mark_completed(client_id, &cmd_id, res.clone());

        AcpResponse {
            jsonrpc: "2.0".to_string(),
            id: req.id,
            result: Some(res),
            error: None,
        }
    }
}

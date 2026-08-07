use cust_config_types::{Config, ModelCapabilities};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelGeneration {
    pub id: u64,
    pub config: Config,
    pub capabilities: ModelCapabilities,
    pub created_at: u64,
}

pub struct GenerationManager {
    current: Arc<Mutex<Arc<ModelGeneration>>>,
    counter: Arc<Mutex<u64>>,
}

impl GenerationManager {
    pub fn new(initial_config: Config) -> Self {
        let model_gen = Arc::new(ModelGeneration {
            id: 1,
            config: initial_config,
            capabilities: ModelCapabilities::default(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        });
        Self {
            current: Arc::new(Mutex::new(model_gen)),
            counter: Arc::new(Mutex::new(1)),
        }
    }

    pub fn current_generation(&self) -> Arc<ModelGeneration> {
        self.current.lock().unwrap().clone()
    }

    pub fn update_generation(&self, new_config: Config) -> Arc<ModelGeneration> {
        let mut cnt = self.counter.lock().unwrap();
        *cnt += 1;
        let model_gen = Arc::new(ModelGeneration {
            id: *cnt,
            config: new_config,
            capabilities: ModelCapabilities::default(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        });
        *self.current.lock().unwrap() = model_gen.clone();
        model_gen
    }
}

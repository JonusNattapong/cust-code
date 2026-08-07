use cust_exec::{CustboxConfig, CustboxMount};
use std::path::PathBuf;

#[test]
fn test_custbox_read_only_path() {
    let config = CustboxConfig {
        mounts: vec![CustboxMount {
            source: PathBuf::from("/tmp/readonly"),
            target: PathBuf::from("/app/readonly"),
            read_only: true,
        }],
        read_only_paths: vec![PathBuf::from("/etc")],
        egress_rules: vec!["allow api.openai.com".to_string()],
        max_memory_mb: Some(512),
    };

    assert!(config.is_path_read_only(&PathBuf::from("/etc/passwd")));
    assert!(config.is_path_read_only(&PathBuf::from("/app/readonly/data.json")));
    assert!(!config.is_path_read_only(&PathBuf::from("/app/writeable/data.json")));
}

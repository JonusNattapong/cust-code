use cust_provider::{ProviderCapabilities, RemoteCompactionSupport};

#[test]
fn test_provider_capabilities_bounds() {
    let openai_cap = ProviderCapabilities::for_provider("openai");
    assert!(openai_cap.namespace_tools);
    assert_eq!(openai_cap.remote_compaction, RemoteCompactionSupport::V2);

    let anthropic_cap = ProviderCapabilities::for_provider("anthropic");
    assert!(!anthropic_cap.image_generation);
    assert_eq!(anthropic_cap.remote_compaction, RemoteCompactionSupport::V1);

    let ollama_cap = ProviderCapabilities::for_provider("ollama");
    assert!(!ollama_cap.namespace_tools);
    assert_eq!(ollama_cap.remote_compaction, RemoteCompactionSupport::Unsupported);
}

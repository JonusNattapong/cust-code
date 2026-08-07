use cust_provider::{OpenAIProvider, ProviderClient, ProviderFailoverGroup};

#[tokio::test]
async fn test_provider_failover_group_creation() {
    let primary = ProviderClient::OpenAI(OpenAIProvider::new(
        "sk-primary".to_string(),
        "gpt-4o".to_string(),
        None,
    ));
    let fallback = ProviderClient::OpenAI(OpenAIProvider::new(
        "sk-fallback".to_string(),
        "gpt-4o-mini".to_string(),
        None,
    ));

    let group = ProviderFailoverGroup::new(primary, vec![fallback]);
    assert_eq!(group.fallbacks.len(), 1);
}

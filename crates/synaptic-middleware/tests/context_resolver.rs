#![cfg(feature = "condenser")]

use synaptic_middleware::condenser::{ContextWindowResolver, DefaultContextWindowResolver};

#[test]
fn test_default_fallback() {
    let resolver = DefaultContextWindowResolver::new(128_000);
    assert_eq!(resolver.resolve("unknown-model", "unknown"), 128_000);
}

#[test]
fn test_register_takes_priority() {
    let resolver = DefaultContextWindowResolver::new(128_000);
    resolver.register("my-model", "my-provider", 200_000);
    assert_eq!(resolver.resolve("my-model", "my-provider"), 200_000);
    // Different provider → fallback
    assert_eq!(resolver.resolve("my-model", "other"), 128_000);
}

#[test]
fn test_multiple_registrations() {
    let resolver = DefaultContextWindowResolver::new(128_000);
    resolver.register("model-a", "prov", 50_000);
    resolver.register("model-b", "prov", 1_000_000);
    assert_eq!(resolver.resolve("model-a", "prov"), 50_000);
    assert_eq!(resolver.resolve("model-b", "prov"), 1_000_000);
    assert_eq!(resolver.resolve("model-c", "prov"), 128_000);
}

#[test]
fn test_register_replaces_existing() {
    let resolver = DefaultContextWindowResolver::new(128_000);
    resolver.register("model-a", "prov", 50_000);
    assert_eq!(resolver.resolve("model-a", "prov"), 50_000);
    resolver.register("model-a", "prov", 200_000);
    assert_eq!(resolver.resolve("model-a", "prov"), 200_000);
}

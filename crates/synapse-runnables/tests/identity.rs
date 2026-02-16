use synapse_core::SynapseError;
use synapse_runnables::{IdentityRunnable, Runnable};

#[tokio::test]
async fn identity_returns_same_value() -> Result<(), SynapseError> {
    let runnable = IdentityRunnable;
    let out = runnable.run("synapse".to_string()).await?;
    assert_eq!(out, "synapse");
    Ok(())
}

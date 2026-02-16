use std::sync::Arc;

use async_trait::async_trait;
use synapse_core::SynapseError;
use synapse_runnables::Runnable;

use synapse_chains::SequentialChain;

struct AppendRunnable(&'static str);

#[async_trait]
impl Runnable<String, String> for AppendRunnable {
    async fn run(&self, input: String) -> Result<String, SynapseError> {
        Ok(format!("{}{}", input, self.0))
    }
}

#[tokio::test]
async fn sequential_chain_runs_all_steps() -> Result<(), SynapseError> {
    let chain = SequentialChain::new(vec![
        Arc::new(AppendRunnable("-a")),
        Arc::new(AppendRunnable("-b")),
    ]);

    let out = chain.run("start".to_string()).await?;
    assert_eq!(out, "start-a-b");
    Ok(())
}

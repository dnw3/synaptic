use std::sync::Arc;

use synapse_core::SynapseError;
use synapse_runnables::Runnable;

pub struct SequentialChain {
    steps: Vec<Arc<dyn Runnable<String, String>>>,
}

impl SequentialChain {
    pub fn new(steps: Vec<Arc<dyn Runnable<String, String>>>) -> Self {
        Self { steps }
    }

    pub async fn run(&self, input: String) -> Result<String, SynapseError> {
        let mut current = input;
        for step in &self.steps {
            current = step.run(current).await?;
        }
        Ok(current)
    }
}

use async_trait::async_trait;
use synapse_core::SynapseError;

#[async_trait]
pub trait Runnable<I, O>: Send + Sync {
    async fn run(&self, input: I) -> Result<O, SynapseError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct IdentityRunnable;

#[async_trait]
impl<T> Runnable<T, T> for IdentityRunnable
where
    T: Send + Sync + 'static,
{
    async fn run(&self, input: T) -> Result<T, SynapseError> {
        Ok(input)
    }
}

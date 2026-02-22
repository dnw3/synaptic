mod azure;
pub(crate) mod chat_model;
pub mod compat;
pub(crate) mod embeddings;

pub use azure::{
    AzureOpenAiChatModel, AzureOpenAiConfig, AzureOpenAiEmbeddings, AzureOpenAiEmbeddingsConfig,
};
pub use chat_model::{OpenAiChatModel, OpenAiConfig};
pub use embeddings::{OpenAiEmbeddings, OpenAiEmbeddingsConfig};

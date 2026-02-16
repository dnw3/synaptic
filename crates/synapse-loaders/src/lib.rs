use synapse_retrieval::Document;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LoaderError {
    #[error("empty document id")]
    EmptyId,
}

#[derive(Debug, Clone)]
pub struct TextLoader {
    id: String,
    content: String,
}

impl TextLoader {
    pub fn new(id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            content: content.into(),
        }
    }

    pub fn load(&self) -> Result<Vec<Document>, LoaderError> {
        if self.id.trim().is_empty() {
            return Err(LoaderError::EmptyId);
        }
        Ok(vec![Document::new(self.id.clone(), self.content.clone())])
    }
}

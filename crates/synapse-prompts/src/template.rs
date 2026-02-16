use std::collections::HashMap;

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PromptError {
    #[error("missing variable: {0}")]
    MissingVariable(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptTemplate {
    template: String,
}

impl PromptTemplate {
    pub fn new(template: impl Into<String>) -> Self {
        Self {
            template: template.into(),
        }
    }

    pub fn render(&self, values: &HashMap<String, String>) -> Result<String, PromptError> {
        let mut output = String::with_capacity(self.template.len());
        let mut rest = self.template.as_str();

        while let Some(start) = rest.find("{{") {
            output.push_str(&rest[..start]);
            let after_start = &rest[start + 2..];
            if let Some(end) = after_start.find("}}") {
                let key = after_start[..end].trim();
                let value = values
                    .get(key)
                    .ok_or_else(|| PromptError::MissingVariable(key.to_string()))?;
                output.push_str(value);
                rest = &after_start[end + 2..];
            } else {
                output.push_str(&rest[start..]);
                rest = "";
                break;
            }
        }

        output.push_str(rest);
        Ok(output)
    }
}

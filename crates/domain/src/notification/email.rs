use async_trait::async_trait;

use crate::identity::Email;

/// One message, already rendered.
///
/// Plain text: this is transactional mail — a reset link and a sentence of
/// context — and HTML would add a rendering concern for no benefit.
#[derive(Debug, Clone)]
pub struct EmailMessage {
    pub to: Email,
    pub subject: String,
    pub body: String,
}

#[async_trait]
pub trait EmailSender: Send + Sync + 'static {
    async fn send(&self, message: EmailMessage) -> Result<(), SendError>;
}

#[derive(Debug, thiserror::Error)]
#[error("the message could not be sent")]
pub struct SendError(#[source] pub Box<dyn std::error::Error + Send + Sync>);

impl SendError {
    pub fn new(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self(Box::new(source))
    }
}

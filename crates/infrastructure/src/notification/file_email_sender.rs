use std::path::PathBuf;

use async_trait::async_trait;
use chrono::Utc;
use domain::notification::{EmailMessage, EmailSender, SendError};
use tokio::io::AsyncWriteExt;

/// Appends messages to a file instead of sending them.
///
/// The development transport — Laravel's `MAIL_MAILER=log`. An SMTP adapter
/// implements the same port later, and no use case changes, because none of
/// them can tell where a message went.
///
/// Opened per message rather than held open: mail is rare, and a long-lived
/// handle would silently keep writing to a rotated-away file.
pub struct FileEmailSender {
    path: PathBuf,
}

impl FileEmailSender {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

#[async_trait]
impl EmailSender for FileEmailSender {
    async fn send(&self, message: EmailMessage) -> Result<(), SendError> {
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(SendError::new)?;
        }

        let rendered = format!(
            "----- {} -----\nTo: {}\nSubject: {}\n\n{}\n\n",
            Utc::now().to_rfc3339(),
            message.to,
            message.subject,
            message.body,
        );

        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .await
            .map_err(SendError::new)?;

        file.write_all(rendered.as_bytes())
            .await
            .map_err(SendError::new)?;

        Ok(())
    }
}

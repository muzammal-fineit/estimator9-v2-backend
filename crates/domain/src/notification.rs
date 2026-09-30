//! Notification context: messages the application sends to people.
//!
//! A port only. Whether that is a log file, SMTP, or a queue is an
//! infrastructure choice, and no use case should be able to tell.

pub mod email;

pub use email::{EmailMessage, EmailSender, SendError};

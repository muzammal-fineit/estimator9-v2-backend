use std::sync::Arc;

use domain::identity::{User, UserId, UserRepository};

use crate::ApplicationError;

pub struct GetUser {
    users: Arc<dyn UserRepository>,
}

impl GetUser {
    pub fn new(users: Arc<dyn UserRepository>) -> Self {
        Self { users }
    }

    pub async fn execute(&self, id: UserId) -> Result<User, ApplicationError> {
        self.users
            .find(id)
            .await?
            .ok_or_else(|| ApplicationError::not_found("user", id))
    }
}

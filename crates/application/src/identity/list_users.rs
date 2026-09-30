use std::sync::Arc;

use domain::identity::{User, UserRepository};

use crate::ApplicationError;

pub struct ListUsers {
    users: Arc<dyn UserRepository>,
}

impl ListUsers {
    pub fn new(users: Arc<dyn UserRepository>) -> Self {
        Self { users }
    }

    pub async fn execute(&self) -> Result<Vec<User>, ApplicationError> {
        Ok(self.users.all().await?)
    }
}

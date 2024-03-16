use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct User {
    email: String,
}

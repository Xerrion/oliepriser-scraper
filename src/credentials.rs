use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) struct Token {
    pub(crate) access_token: String,
    pub(crate) token_type: String,
}

pub(crate) struct Credentials {
    pub(crate) client_id: String,
    pub(crate) client_secret: String,
    pub(crate) token: Token,
}

impl Credentials {
    pub(crate) fn new(client_id: String, client_secret: String) -> Self {
        Self {
            client_id,
            client_secret,
            token: Token {
                access_token: "".to_string(),
                token_type: "".to_string(),
            },
        }
    }
}

use black_jack::registry::client::cassino as _cassino;
use rio_rs::{client::Client as _Client, cluster::storage::http::HttpMembersStorage};

#[derive(uniffi::Object)]
pub struct Client(_Client<HttpMembersStorage>);

#[uniffi::export]
impl Client {
    #[uniffi::constructor]
    pub fn new(rendevouz_http_addr: String) -> Client {
        let storage = HttpMembersStorage {
            remote_address: rendevouz_http_addr,
        };
        let client = _Client::new(storage);
        Client(client)
    }
}

pub mod cassino {
    use black_jack::messages::JoinGame;

    use super::*;
    pub async fn send_join_message(
        client: &mut Client,
        object_id: String,
        user_id: String,
    ) -> (String, Vec<String>) {
        let client_ = &mut client.0;
        let msg = JoinGame { user_id };
        let resp = _cassino::send_join_game(client_, &object_id, &msg)
            .await
            .unwrap();
        (resp.table_id, resp.user_ids)
    }
}

uniffi::setup_scaffolding!();

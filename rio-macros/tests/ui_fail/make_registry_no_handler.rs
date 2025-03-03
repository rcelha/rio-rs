use rio_rs::prelude::*;
use serde::{Deserialize, Serialize};

use rio_macros::make_registry;

#[derive(Default, Debug, WithId, TypeName)]
struct TestService {
    id: String,
}

impl ServiceObjectStateLoad for TestService {}
impl ServiceObject for TestService {}

#[derive(TypeName, Message, Debug, Deserialize, Serialize)]
pub struct Ping {
    pub ping_id: String,
}

#[derive(TypeName, Message, Debug, Deserialize, Serialize)]
pub struct Pong {
    pub ping_id: String,
}

make_registry! {
    TestService: [
        Ping => (Pong, NoopError),
    ]
}

fn main() {
    let _registry = server::registry();
}

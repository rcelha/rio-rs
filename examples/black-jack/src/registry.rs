use rio_rs::prelude::*;

use crate::{
    messages::{JoinGame, JoinGameResponse, PlayerCommand, PlayerCommandResponse},
    services::{cassino::Cassino, table::GameTable},
};

#[allow(unused)]
type LifecycleReturn = ();

make_registry! {
    Cassino: [
        LifecycleMessage => (LifecycleReturn, ServiceObjectLifeCycleError),
        JoinGame => (JoinGameResponse, NoopError)
    ],
    GameTable: [
        LifecycleMessage => (LifecycleReturn, ServiceObjectLifeCycleError),
        JoinGame => (JoinGameResponse, NoopError),
        PlayerCommand => (PlayerCommandResponse, NoopError)
    ]
}

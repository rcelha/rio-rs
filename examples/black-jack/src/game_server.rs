use bevy::{app::AppExit, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{card::Card, hand::Hand, table::Table};

struct TurnTimer(Timer);
struct PoolTimer(Timer);
type PlayerList = Vec<String>;
type GameResults = Vec<Winner>;
type CurrentPlayer = String;
type NextPlayer = String;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum TableState {
    Wait,
    Dealing,
    Running,
    Settling,
    Done,
}

fn turn_end(
    time: Res<Time>,
    mut turn_timer: ResMut<TurnTimer>,
    mut table_state: ResMut<TableState>,
    current_player: ResMut<CurrentPlayer>,
    player_list: ResMut<PlayerList>,
) {
    if !turn_timer.0.tick(time.delta()).just_finished() {
        return;
    }

    if table_state.as_ref() != &TableState::Running {
        return;
    }

    if current_player.as_ref() == "" {
        *table_state = TableState::Settling;
        return;
    }

    // Force play: Stand
    util_finish_player_turn(turn_timer, player_list, current_player);
}

fn finish_join_stage(
    mut turn_timer: ResMut<TurnTimer>,
    mut table_state: ResMut<TableState>,
    mut current_player: ResMut<CurrentPlayer>,
    mut table: ResMut<Table>,
    player_list: ResMut<PlayerList>,
    // query: Query<&Table>,
) {
    if table_state.as_ref() != &TableState::Wait {
        return;
    }

    if !player_list.is_changed() {
        return;
    }

    if player_list.len() < 3 {
        return;
    }

    *table_state = TableState::Running;
    *current_player = player_list.first().unwrap().clone();
    for _ in 0..2 {
        for card in [
            Card::Two,
            Card::Three,
            Card::Four,
            Card::Five,
            Card::Six,
            Card::Seven,
            Card::Eight,
            Card::Nine,
            Card::Jack,
            Card::Queen,
            Card::King,
            Card::Ace,
        ] {
            table.deck.push(card);
        }
    }
    for i in player_list.iter() {
        table.players.insert(i.clone(), Hand::new());
        table.deal(i);
        table.deal(i);
    }
    table.deal_to_dealer();
    table.deal_to_dealer();
    turn_timer.0.reset();
}

fn settle(
    mut table_state: ResMut<TableState>,
    mut table: ResMut<Table>,
    player_list: Res<PlayerList>,
    mut game_results: ResMut<GameResults>,
    // query: Query<&Table>,
) {
    // Only run this once per table_state change
    if !table_state.is_changed() {
        return;
    }

    if table_state.as_ref() != &TableState::Settling {
        return;
    }

    if table.dealer.value() < 21 {
        table.deal_to_dealer();
    }

    for i in player_list.iter() {
        let player_hand = table.players.get(i).expect("Player not found");
        let result = Table::settle(&table.dealer, player_hand);

        let result = match result {
            -1 => Winner::Dealer,
            1 => Winner::Player,
            _ => Winner::Tie,
        };
        game_results.push(result);
    }
    *table_state = TableState::Done;
}

fn util_finish_player_turn(
    mut turn_timer: ResMut<TurnTimer>,
    player_list: ResMut<PlayerList>,
    mut current_player: ResMut<CurrentPlayer>,
) {
    let current_player_idx = player_list
        .iter()
        .position(|x| x == current_player.as_ref())
        .unwrap_or(1_000);

    let empty_player = "".to_string();
    let next_player = player_list
        .get(current_player_idx + 1)
        .unwrap_or(&empty_player);
    *current_player = next_player.to_owned();

    turn_timer.0.reset();
}

fn pool_commands(
    time: Res<Time>,
    mut timer: ResMut<PoolTimer>,
    turn_timer: ResMut<TurnTimer>,
    mut player_list: ResMut<PlayerList>,
    table_state: Res<TableState>,
    mut table: ResMut<Table>,
    current_player: ResMut<CurrentPlayer>,
    game_results: Res<GameResults>,
    command_receiver: Res<crossbeam_channel::Receiver<GameServerRequest>>,
    message_sender: Res<crossbeam_channel::Sender<GameServerResponse>>,
    mut exit: EventWriter<AppExit>,
    // query: Query<&Table>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        let req = command_receiver.try_recv();
        if req.is_err() {
            return;
        }
        println!(
            "request received: {:?} {} {:?}",
            req,
            current_player.as_ref(),
            table_state
        );
        let res = match req {
            Ok(GameServerRequest::Admin(AdminCommand::Quit)) => {
                exit.send(AppExit);
                GameServerResponse::Empty
            }
            Ok(GameServerRequest::Player(_, PlayerCommand::Stand)) => {
                util_finish_player_turn(turn_timer, player_list, current_player);
                GameServerResponse::Empty
            }
            Ok(GameServerRequest::Player(player_id, PlayerCommand::Hit)) => {
                println!("hit {:?} {:?}", player_id, current_player);
                if player_id != *current_player {
                    println!("hit? NO");
                    message_sender.send(GameServerResponse::Empty).ok();
                    return;
                }
                table.deal(&player_id);

                util_finish_player_turn(turn_timer, player_list, current_player);
                GameServerResponse::Empty
            }
            Ok(GameServerRequest::Player(player_id, PlayerCommand::Join)) => {
                player_list.push(player_id.clone());
                GameServerResponse::Empty
            }
            Ok(GameServerRequest::Player(_, PlayerCommand::ListPlayers)) => {
                GameServerResponse::Players(player_list.clone())
            }
            Ok(GameServerRequest::Player(_, PlayerCommand::GetState)) => {
                let mut hands = vec![];
                let results = game_results.iter().cloned().collect();
                let player_list = player_list.iter().cloned().collect();

                if table_state.as_ref() == &TableState::Done {
                    for (_, player) in table.players.iter() {
                        let hand: Vec<u8> = player.0.iter().map(|x| x.value()).collect();
                        hands.push(hand);
                    }
                    let hand: Vec<u8> = table.dealer.0.iter().map(|x| x.value()).collect();
                    hands.push(hand);
                }

                GameServerResponse::State(
                    table_state.clone(),
                    current_player.clone(),
                    player_list,
                    hands,
                    results,
                )
            }
            _ => GameServerResponse::Empty,
        };
        message_sender.send(res).ok();
    }
}

/// Print the stats of friendly players when they change
fn check_res_changed(
    players: Res<PlayerList>,
    table_state: Res<TableState>,
    table: Res<Table>,
    current_player: Res<CurrentPlayer>,
    game_results: Res<GameResults>,
    message_sender: Res<crossbeam_channel::Sender<GameServerStates>>,
) {
    if players.is_changed()
        || table_state.is_changed()
        || table.is_changed()
        || current_player.is_changed()
        || game_results.is_changed()
    {
        let mut hands = vec![];
        let results = game_results.iter().cloned().collect();

        if table_state.as_ref() == &TableState::Done {
            for (_, player) in table.players.iter() {
                let hand: Vec<u8> = player.0.iter().map(|x| x.value()).collect();
                hands.push(hand);
            }
            let hand: Vec<u8> = table.dealer.0.iter().map(|x| x.value()).collect();
            hands.push(hand);
        }

        let msg = GameServerResponse::State(
            table_state.clone(),
            current_player.clone(),
            players.clone(),
            hands,
            results,
        );
        message_sender.send(GameServerStates(vec![msg])).ok();
    }
}

// Not implemented: Double, Split, Surrender
#[derive(Debug, Serialize, Deserialize)]
pub enum PlayerCommand {
    Join,
    Stand,
    Hit,
    ListPlayers,
    GetState,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AdminCommand {
    Quit,
}

// Incoming messages
#[derive(Debug, Serialize, Deserialize)]
pub enum GameServerRequest {
    // Command with player ID
    // The application server must validate the player ID beforehand
    Player(String, PlayerCommand),
    Admin(AdminCommand),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Winner {
    Dealer,
    Tie,
    Player,
}

// Outbound messages
#[derive(Debug, Serialize, Deserialize)]
pub enum GameServerResponse {
    // State, current player, player list, hands, winners
    State(TableState, String, Vec<String>, Vec<Vec<u8>>, Vec<Winner>),
    Players(Vec<String>),
    Empty,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GameServerStates(pub Vec<GameServerResponse>);

pub struct GameServerConfig {
    pub turn_duration_in_seconds: f32,
}

impl Default for GameServerConfig {
    fn default() -> Self {
        GameServerConfig {
            turn_duration_in_seconds: 30.0,
        }
    }
}

pub struct GameServer {
    pub config: GameServerConfig,
}

impl Plugin for GameServer {
    fn build(&self, app: &mut App) {
        let table = Table::new();
        let player_list: PlayerList = vec![];
        let current_player: CurrentPlayer = "".to_string();
        let next_player: NextPlayer = "".to_string();

        app.insert_resource(PoolTimer(Timer::from_seconds(0.1, true)))
            .insert_resource(TurnTimer(Timer::from_seconds(
                self.config.turn_duration_in_seconds,
                true,
            )))
            .insert_resource(current_player)
            .insert_resource(next_player)
            .insert_resource(table)
            .insert_resource(player_list)
            .insert_resource(TableState::Wait)
            .insert_resource(GameResults::new())
            .add_system(check_res_changed)
            .add_system(pool_commands)
            .add_system(finish_join_stage)
            .add_system(settle)
            .add_system(turn_end);
    }
}

pub fn build_app(
    tx: crossbeam_channel::Sender<GameServerResponse>,
    rx: crossbeam_channel::Receiver<GameServerRequest>,
    changes_tx: crossbeam_channel::Sender<GameServerStates>,
    config: Option<GameServerConfig>,
) -> App {
    let config = config.unwrap_or_default();
    let mut app = App::new();
    app.insert_resource(rx);
    app.insert_resource(tx);
    app.insert_resource(changes_tx);
    app.add_plugins(MinimalPlugins)
        .add_plugin(GameServer { config });
    app
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn sanity_check() {
        let (res_tx, _) = crossbeam_channel::bounded(1_000);
        let (_, req_rx) = crossbeam_channel::bounded(1_000);
        let (changes_tx, _) = crossbeam_channel::bounded(1_000);

        let _app = build_app(res_tx, req_rx, changes_tx, None);
    }

    #[derive(Clone)]
    struct AppTest {
        pub response_tx: crossbeam_channel::Sender<GameServerResponse>,
        pub response_rx: crossbeam_channel::Receiver<GameServerResponse>,
        pub request_tx: crossbeam_channel::Sender<GameServerRequest>,
        pub request_rx: crossbeam_channel::Receiver<GameServerRequest>,
        pub changes_tx: crossbeam_channel::Sender<GameServerStates>,
    }

    impl AppTest {
        fn new() -> AppTest {
            let (response_tx, response_rx) = crossbeam_channel::bounded(1_000);
            let (request_tx, request_rx) = crossbeam_channel::bounded(1_000);
            let (changes_tx, _changes_rx) = crossbeam_channel::bounded(1_000);
            AppTest {
                response_tx,
                response_rx,
                request_tx,
                request_rx,
                changes_tx,
            }
        }

        fn app(self) -> App {
            build_app(
                self.response_tx,
                self.request_rx,
                self.changes_tx,
                Some(GameServerConfig {
                    turn_duration_in_seconds: 1.0,
                }),
            )
        }

        fn request(&self, req: GameServerRequest) -> GameServerResponse {
            self.request_tx.send(req).unwrap();
            self.response_rx.recv().unwrap()
        }
    }

    #[test]
    fn test_commands() {
        let test_app = AppTest::new();

        let inner_test_app = test_app.clone();
        let join_handler = std::thread::spawn(move || {
            let mut app = inner_test_app.app();
            app.run();
        });

        let p1 = "player-1".to_string();
        let p2 = "player-2".to_string();
        let p3 = "player-3".to_string();

        test_app.request(GameServerRequest::Player(p1.clone(), PlayerCommand::Join));
        let list_players_resp = test_app.request(GameServerRequest::Player(
            p1.clone(),
            PlayerCommand::ListPlayers,
        ));
        match list_players_resp {
            GameServerResponse::Players(x) => assert_eq!(x.len(), 1),
            _ => panic!("no players"),
        };

        test_app.request(GameServerRequest::Player(p2.clone(), PlayerCommand::Join));
        let list_players_resp = test_app.request(GameServerRequest::Player(
            p2.clone(),
            PlayerCommand::ListPlayers,
        ));
        match list_players_resp {
            GameServerResponse::Players(x) => assert_eq!(x.len(), 2),
            _ => panic!("no players"),
        };

        let table_state = test_app.request(GameServerRequest::Player(
            p2.clone(),
            PlayerCommand::GetState,
        ));
        match table_state {
            GameServerResponse::State(TableState::Wait, _, _, _, _) => (),
            _ => panic!("no players"),
        };

        test_app.request(GameServerRequest::Player(p3.clone(), PlayerCommand::Join));
        std::thread::sleep(std::time::Duration::from_millis(100));
        let table_state = test_app.request(GameServerRequest::Player(
            p3.clone(),
            PlayerCommand::GetState,
        ));
        match table_state {
            GameServerResponse::State(TableState::Running, current_player, _, _, _) => {
                assert_eq!(current_player, p1)
            }
            x => panic!("Status mismatch {:#?}", x),
        };

        test_app.request(GameServerRequest::Player(p1.clone(), PlayerCommand::Hit));
        test_app.request(GameServerRequest::Player(p2.clone(), PlayerCommand::Stand));

        // Forces player3 to stand
        std::thread::sleep(std::time::Duration::from_secs(4));

        let table_state = test_app.request(GameServerRequest::Player(
            p3.clone(),
            PlayerCommand::GetState,
        ));
        match table_state {
            GameServerResponse::State(
                TableState::Done,
                current_player,
                player_list,
                hands,
                results,
            ) => {
                println!("hands {:?}", player_list);
                println!("hands {:?}", hands);
                println!("results {:?}", results);
                assert_eq!(current_player, "".to_string())
            }
            x => panic!("Status mismatch {:#?}", x),
        };

        test_app.request(GameServerRequest::Admin(AdminCommand::Quit));
        join_handler.join().unwrap();
    }

    #[test]
    fn test_fullgame_with_all_players_standing_on_their_round() {
        let test_app = AppTest::new();

        let inner_test_app = test_app.clone();
        let join_handler = std::thread::spawn(move || {
            let mut app = inner_test_app.app();
            app.run();
        });

        let p1 = "player-1".to_string();
        let p2 = "player-2".to_string();
        let p3 = "player-3".to_string();

        test_app.request(GameServerRequest::Player(p1.clone(), PlayerCommand::Join));
        test_app.request(GameServerRequest::Player(p2.clone(), PlayerCommand::Join));
        test_app.request(GameServerRequest::Player(p3.clone(), PlayerCommand::Join));

        std::thread::sleep(std::time::Duration::from_millis(100));
        test_app.request(GameServerRequest::Player(p1.clone(), PlayerCommand::Stand));
        test_app.request(GameServerRequest::Player(p2.clone(), PlayerCommand::Stand));
        test_app.request(GameServerRequest::Player(p3.clone(), PlayerCommand::Stand));

        // Forces player3 to stand
        std::thread::sleep(std::time::Duration::from_secs(4));

        let table_state = test_app.request(GameServerRequest::Player(
            p3.clone(),
            PlayerCommand::GetState,
        ));
        match table_state {
            GameServerResponse::State(
                TableState::Done,
                current_player,
                player_list,
                hands,
                results,
            ) => {
                println!("hands {:?}", player_list);
                println!("hands {:?}", hands);
                println!("results {:?}", results);
                assert_eq!(current_player, "".to_string())
            }
            x => panic!("Status mismatch {:#?}", x),
        };

        test_app.request(GameServerRequest::Admin(AdminCommand::Quit));
        join_handler.join().unwrap();
    }
}

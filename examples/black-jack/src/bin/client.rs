use black_jack::game_server::{
    GameServerRequest, GameServerResponse, GameServerStates, TableState,
};
use black_jack::messages::{JoinGame, JoinGameResponse, PlayerCommand, PlayerCommandResponse};

use clap::Parser;
use futures::{pin_mut, StreamExt};
use rio_rs::cluster::storage::sql::SqlMembersStorage;
use rio_rs::prelude::*;
use std::process::exit;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::time::sleep;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Opts {
    pub player_name: String,
    #[arg(
        short,
        long,
        default_value = "sqlite:///tmp/black-jack-membership.sqlite3?mode=rwc"
    )]
    pub cluster_membership_provider_conn: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let options = Opts::parse();
    let user_id = options.player_name.clone();

    let pool = SqlMembersStorage::pool()
        .max_connections(50)
        .connect(&options.cluster_membership_provider_conn)
        .await?;
    let members_storage = SqlMembersStorage::new(pool);

    sleep(Duration::from_secs(1)).await;

    members_storage.prepare().await;
    let servers = members_storage.active_members().await;
    println!("server: {:?}", servers);

    let mut client = ClientBuilder::new()
        .members_storage(members_storage)
        .build()?;

    let msg = JoinGame {
        user_id: user_id.clone(),
    };
    let table: JoinGameResponse = client.send("Cassino", "*", &msg).await?;
    println!("Table #{:#?}", table);

    let stdin = tokio::io::stdin();
    let mut buffered_stdin = BufReader::new(stdin);
    let mut command = "h".to_string();

    let mut inner_client = client.clone();
    let inner_table_id = table.table_id.clone();

    let subscription = inner_client
        .subscribe::<GameServerStates>("GameTable", inner_table_id)
        .await?;
    pin_mut!(subscription);
    while let Some(data) = subscription.next().await {
        match data {
            Ok(ref v) => {
                //
                match v.0.last() {
                    Some(GameServerResponse::State(
                        table_state,
                        current_player,
                        player_list,
                        hands,
                        winners,
                    )) => {
                        if table_state == &TableState::Wait {
                            println!("Waiting for players to join");
                            continue;
                        }

                        if table_state == &TableState::Settling {
                            println!("Settling te scores");
                            continue;
                        }

                        if table_state == &TableState::Done {
                            println!("Game is over");
                            println!("---");
                            let dealer_total: u8 = hands.last().unwrap().iter().sum();
                            println!(
                                "The DEALER hand is {:?} ({})",
                                hands.last().unwrap(),
                                dealer_total
                            );

                            for i in 0..player_list.len() {
                                let i_player_id = player_list.get(i).unwrap();
                                let i_hand = hands.get(i).unwrap();
                                let i_total: u8 = i_hand.iter().sum();
                                let i_result = winners.get(i).unwrap();

                                println!("---");
                                println!("The {} hand is {:?} ({})", i_player_id, i_hand, i_total);
                                match i_result {
                                    black_jack::game_server::Winner::Dealer => {
                                        println!("The Dealer won agains {}", i_player_id);
                                    }
                                    black_jack::game_server::Winner::Player => {
                                        println!("The player {} won", i_player_id);
                                    }
                                    black_jack::game_server::Winner::Tie => {
                                        println!(
                                            "The player {} has tied with the DEALER",
                                            i_player_id
                                        );
                                    }
                                }
                            }
                            exit(0);
                        }

                        if current_player.is_empty() {
                            println!("The DEALER is playing");
                        } else if current_player != &user_id {
                            println!("{} is currently playing", current_player);
                        } else {
                            println!("It is your turn baby");
                            loop {
                                println!("Insert your command:");
                                println!("  - `!` Hit it");
                                println!("  - `x` Stand");

                                command.clear();
                                let timeout_result =
                                    tokio::time::timeout(Duration::from_secs(9), async {
                                        buffered_stdin.read_line(&mut command).await.ok();
                                    })
                                    .await;

                                if timeout_result.is_err() {
                                    println!("Timeout, you 'standed'");
                                    break;
                                }

                                let command = match command.trim() {
                                    "s" => black_jack::game_server::PlayerCommand::GetState,
                                    "!" => black_jack::game_server::PlayerCommand::Hit,
                                    "x" => black_jack::game_server::PlayerCommand::Stand,
                                    _ => {
                                        continue;
                                    }
                                };
                                let msg = PlayerCommand(GameServerRequest::Player(
                                    user_id.clone(),
                                    command,
                                ));
                                let resp: PlayerCommandResponse = client
                                    .send("GameTable", &table.table_id, &msg)
                                    .await
                                    .unwrap();
                                println!("Server Response {:#?}", resp);
                                break;
                            }
                        }
                    }
                    _ => (),
                }
            }
            _ => {}
        };
    }

    Ok(())
}

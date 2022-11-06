use black_jack::game_server::GameServerRequest;
use black_jack::messages::{JoinGame, JoinGameResponse, PlayerCommand, PlayerCommandResponse};

use clap::Parser;
use rio_rs::cluster::storage::sql::SqlMembersStorage;
use rio_rs::prelude::*;
use std::io::Write;
use std::time::Duration;
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

    members_storage.migrate().await;
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

    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut command = "h".to_string();

    loop {
        print!("Insert your command ('h' for help): ");
        stdout.flush()?;
        command.clear();
        stdin.read_line(&mut command)?;

        let command = match command.trim() {
            "s" => black_jack::game_server::PlayerCommand::GetState,
            "!" => black_jack::game_server::PlayerCommand::Hit,
            "x" => black_jack::game_server::PlayerCommand::Stand,
            "q" => break,
            _ => {
                println!("Help: ");
                println!("");
                println!("- h       Print this message");
                println!("- s       Get the table's current state");
                println!("- !       Hit it");
                println!("- x       Stand");
                println!("- q       Exit program");
                continue;
            }
        };

        let msg = PlayerCommand(GameServerRequest::Player(user_id.clone(), command));
        let resp: PlayerCommandResponse = client
            .send("GameTable", &table.table_id, &msg)
            .await
            .unwrap();
        println!("Server Response {:#?}", resp);
    }

    Ok(())
}

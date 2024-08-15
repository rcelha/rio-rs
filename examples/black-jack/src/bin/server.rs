use black_jack::rio_server::build_server;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Opts {
    pub port: u16,
    #[arg(
        short,
        long,
        default_value = "sqlite:///tmp/black-jack-membership.sqlite3?mode=rwc"
    )]
    pub cluster_membership_provider_conn: String,

    #[arg(
        short,
        long,
        default_value = "sqlite:///tmp/black-jack-placement.sqlite3?mode=rwc"
    )]
    pub object_placement_provider_conn: String,

    #[arg(
        short,
        long,
        default_value = "sqlite:///tmp/black-jack-state.sqlite3?mode=rwc"
    )]
    pub sql_state_conn: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let options = Opts::parse();
    let mut server = build_server(
        &options.port.to_string(),
        &options.cluster_membership_provider_conn,
        &options.object_placement_provider_conn,
        &options.sql_state_conn,
    )
    .await?;
    server.bind().await?;
    server.run().await?;
    Ok(())
}

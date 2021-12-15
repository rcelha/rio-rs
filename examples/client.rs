use example_utils::messages;
use tokio::io::{AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;
use tokio_util::codec::LinesCodec;

#[tokio::main]
async fn main() {
    let addr = "0.0.0.0:5000";
    let mut stream = TcpStream::connect(&addr).await.unwrap();
    println!("Connected to: {}", addr);
    let msg = messages::Metric {
        tags: "eu-west-1".to_string(),
        value: 100,
    };
    stream
        .write_all(bincode::serialize(&msg).unwrap().as_ref())
        .await
        .unwrap();
}

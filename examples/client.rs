use example_utils::messages;
use futures::SinkExt;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::io::{AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_stream::StreamExt;
use tokio_util::codec::LinesCodec;
use tokio_util::codec::{BytesCodec, Framed};

#[tokio::main]
async fn main() {
    let addr = "0.0.0.0:5000";
    let stream = TcpStream::connect(&addr).await.unwrap();
    let mut frames = Framed::new(stream, BytesCodec::new());

    println!("Connected to: {}", addr);
    let msg = messages::Metric {
        tags: "eu-west-1".to_string(),
        value: 100,
    };
    let ser_msg = bincode::serialize(&msg).unwrap();
    frames.send(ser_msg.into()).await.unwrap();
    println!("msg sent");

    if let Some(Ok(frame)) = frames.next().await {
        let message: messages::MetricResponse = bincode::deserialize(&frame).unwrap();
        println!("Decoded message: {:?}", message);
    }
}

use example_utils::messages;
use futures::sink::SinkExt;
use tokio::io::BufReader;
use tokio::net::{TcpListener, TcpStream};
use tokio_stream::StreamExt;
use tokio_util::codec::{BytesCodec, Framed};

async fn handle_client(stream: TcpStream) {
    println!("client connected");
    let stream = BufReader::new(stream);
    let mut frames = Framed::new(stream, BytesCodec::new());

    while let Some(Ok(frame)) = frames.next().await {
        let message: messages::Metric = bincode::deserialize(&frame).unwrap();
        println!("Decoded message: {:?}", message);

        let response = messages::MetricResponse {
            avg: 100,
            max: 1000,
            min: 10,
            sum: 100000,
        };
        let ser_response = bincode::serialize(&response).unwrap();
        frames.send(ser_response.into()).await.unwrap();
    }
    println!("client disconnected");
}

#[tokio::main]
async fn main() {
    let addr = "0.0.0.0:5000";
    let listener = TcpListener::bind(&addr).await.unwrap();
    println!("Listening on: {}", addr);
    loop {
        let (stream, _) = listener.accept().await.unwrap();
        tokio::spawn(async move { handle_client(stream).await });
    }
}

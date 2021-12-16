use example_utils::{grains, messages};
use futures::sink::SinkExt;
use rio_rs::Registry;
use tokio::io::BufReader;
use tokio::net::{TcpListener, TcpStream};
use tokio_stream::StreamExt;
use tokio_util::codec::{BytesCodec, Framed};

async fn handle_client(stream: TcpStream) {
    println!("client connected");
    let stream = BufReader::new(stream);
    let mut frames = Framed::new(stream, BytesCodec::new());

    let mut registry = Registry::new();
    registry.add_handler::<grains::MetricAggregator, messages::Metric>();
    let obj = grains::MetricAggregator::default();
    registry.add("instance-1".to_string(), obj).await;

    while let Some(Ok(frame)) = frames.next().await {
        let response = registry
            .send("MetricAggregator", "instance-1", "Metric", &frame)
            .await
            .unwrap();
        frames.send(response.into()).await.unwrap();
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

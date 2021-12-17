use example_utils::messages;
use futures::SinkExt;
use rio_rs::{RequestEnvelope, ResponseEnvelope};
use tokio::net::TcpStream;
use tokio_stream::StreamExt;
use tokio_util::codec::{BytesCodec, Framed};

#[tokio::main]
async fn main() {
    let addr = "0.0.0.0:5000";
    let stream = TcpStream::connect(&addr).await.unwrap();
    let mut frames = Framed::new(stream, BytesCodec::new());

    for i in 1..11 {
        println!("Connected to: {}", addr);
        let payload = messages::Metric {
            tags: "eu-west-1".to_string(),
            value: 100 * i,
        };
        let request = RequestEnvelope::new(
            "MetricAggregator".to_string(),
            "instance-1".to_string(),
            "Metric".to_string(),
            bincode::serialize(&payload).unwrap(),
        );
        let ser_request = bincode::serialize(&request).unwrap();
        frames.send(ser_request.into()).await.unwrap();
        println!("msg sent");

        if let Some(Ok(frame)) = frames.next().await {
            let message: ResponseEnvelope = bincode::deserialize(&frame).unwrap();
            let body: messages::MetricResponse = bincode::deserialize(&message.body.unwrap()).unwrap();
            println!("Decoded message: {:#?}", body);
        }
    }
}

use std::sync::Arc;

use example_utils::{grains, messages};
use futures::sink::SinkExt;
use rio_rs::{Registry, RequestEnvelope, ResponseEnvelope};
use tokio::io::BufReader;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tokio_stream::StreamExt;
use tokio_util::codec::{BytesCodec, Framed};

async fn handle_client(registry: Arc<RwLock<Registry>>, stream: TcpStream) {
    println!("client connected");
    let stream = BufReader::new(stream);
    let mut frames = Framed::new(stream, BytesCodec::new());

    while let Some(Ok(frame)) = frames.next().await {
        let request_envelope: RequestEnvelope = bincode::deserialize(&frame).unwrap();

        if !registry
            .read()
            .await
            .has(
                &request_envelope.handler_type.clone(),
                &request_envelope.handler_id.clone(),
            )
            .await
        {
            registry
                .write()
                .await
                .insert_object(
                    request_envelope.handler_type.clone(),
                    request_envelope.handler_id.clone(),
                )
                .await;
        }

        let response = registry
            .write()
            .await
            .send(
                &request_envelope.handler_type,
                &request_envelope.handler_id,
                &request_envelope.message_type,
                &request_envelope.payload,
            )
            .await;

        let response_envelope = match response {
            Ok(body) => ResponseEnvelope::new(body),
            Err(err) => ResponseEnvelope::from(err),
        };
        let ser_response = bincode::serialize(&response_envelope).unwrap();
        frames.send(ser_response.into()).await.unwrap();
    }
    println!("client disconnected");
}

#[tokio::main]
async fn main() {
    let addr = "0.0.0.0:5000";
    let listener = TcpListener::bind(&addr).await.unwrap();

    let mut registry = Registry::new();
    registry.add_handler::<grains::MetricAggregator, messages::Metric>();
    let registry = Arc::new(RwLock::new(registry));

    println!("Listening on: {}", addr);
    loop {
        let (stream, _) = listener.accept().await.unwrap();
        let inner_registry = Arc::clone(&registry);
        tokio::spawn(async move { handle_client(inner_registry, stream).await });
    }
}

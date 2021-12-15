use example_utils::Hello;
use tokio::net::{TcpListener, TcpStream};
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;
use tokio_util::codec::LinesCodec;

async fn handle_client(stream: TcpStream) {
    println!("client connected");
    let mut lines = Framed::new(stream, LinesCodec::new());
    while let Some(value) = lines.next().await {
        let the_value = value.unwrap();
        println!("Got {}", the_value);
        let byte_value = the_value.as_bytes();
        let decoded_value: Hello = bincode::deserialize(&byte_value).unwrap();
        println!("Got {:?}", decoded_value);
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

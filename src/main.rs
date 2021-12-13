use tokio::net::{TcpListener, TcpStream};
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;
use tokio_util::codec::LinesCodec;

async fn handle_client(stream: TcpStream) {
    let mut lines = Framed::new(stream, LinesCodec::new());
    while let Some(value) = lines.next().await {
        println!("Got {}", value.unwrap());
    }
}

#[tokio::main]
async fn main() {
    let addr = "0.0.0.0:5000";
    let listener = TcpListener::bind(&addr).await.unwrap();
    println!("Listening on: {}", addr);
    println!("main");
    loop {
        let (stream, _) = listener.accept().await.unwrap();
        tokio::spawn(async move { handle_client(stream).await });
    }
}

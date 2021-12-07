use rio_rs::*;
use serde::{Deserialize, Serialize};
use std::{
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
};
use threadpool::ThreadPool;

fn handle_connection(mut stream: TcpStream) {
    let mut registry = Registry::new();
    registry.add("1".to_string(), messages::Human {});
    registry.add("2".to_string(), messages::Human {});
    registry.add_handler::<messages::Human, messages::HiMessage>();
    registry.add_handler::<messages::Human, messages::GoodbyeMessage>();

    let reader_stream = stream.try_clone().expect("Error creating reader");
    let reader = BufReader::new(reader_stream);

    println!("starting a new client");
    for line in reader.lines() {
        if line.is_err() {
            break;
        }
        let line = line.unwrap();
        let mut members = line.splitn(4, ";");
        let grain_type = members.next().unwrap();
        let grain_id = members.next().unwrap();
        let message_type = members.next().unwrap();
        let msg = members.next().unwrap();

        println!("i will read this");
        println!("{:?}", line);
        let result = registry
            .send(grain_type, grain_id, message_type, msg.as_bytes())
            .unwrap();
        stream.write(&result).expect("Error writing");
        stream.write(b"\n");
    }
}

fn main() {
    let listener = TcpListener::bind("0.0.0.0:5000").unwrap();
    let pool = ThreadPool::new(4);

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        pool.execute(move || {
            handle_connection(stream);
        });
    }
}

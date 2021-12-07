use rio_rs::*;

use rio_rs::*;
use serde::{Deserialize, Serialize};
use std::{
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
};
use threadpool::ThreadPool;

fn main() {
    let mut stream = TcpStream::connect("0.0.0.0:5000").unwrap();
    let mut reader = BufReader::new(stream.try_clone().expect("create reader"));

    let msg = messages::HiMessage {
        name: "Foo".to_string(),
    };
    let serialized_msg = bincode::serialize(&msg).unwrap();
    stream.write(b"Human;2;HiMessage;").unwrap();
    stream.write(&serialized_msg).unwrap();
    stream.write(b"\n").unwrap();

    let resp = reader.lines().next();
    println!("response {:?}", resp);
    let resp = resp.unwrap().unwrap();
    let parsed_resp: messages::HiMessage = bincode::deserialize(&resp.as_bytes()).unwrap();
    println!("response {:?}", parsed_resp);
    println!("done");

    stream.shutdown(std::net::Shutdown::Both).unwrap();
}

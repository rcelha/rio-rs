# Black Jack

This is an example app using Bevy as a game server.

A few interesting things you can see here:

- State storage
- Background tasks
- Object-to-object communication

## Running

You can spawn multiple servers with the command:

```sh
cargo run --bin server -- 50999
```

Each new client will get the first seat available, if the table is full
the server will create a new one.

To run the client:

```sh
cargo run --bin client PLAYER_ID
```

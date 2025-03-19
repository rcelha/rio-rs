# Changelog

All notable changes to this project will be documented in this file.

## [unreleased]

### 🚀 Features

- Backoff retry for client connection [382ccf6](https://github.com/rcelha/rio-rs/commit/382ccf60b9adadf648fc289d26163e45d707c2dd) 

### 📚 Documentation

- Update changelog format to include commit [df3745e](https://github.com/rcelha/rio-rs/commit/df3745ef2de01da2bcc6f240dd2fae1302d22dd5) 

### ⚙️ Miscellaneous Tasks

- Update changelog [0f563b2](https://github.com/rcelha/rio-rs/commit/0f563b281b959c4fe80dc9d213210c125c1f7e5d) 

## [0.3.2] - 2025-03-17

### 🚀 Features

- BREAKING - Revert generics on state to be at the trait level [e07e495](https://github.com/rcelha/rio-rs/commit/e07e495bd70d59ec567d5e1986a30e9a6aa8a4b7) 

### 🚜 Refactor

- Massive renaming [85985a1](https://github.com/rcelha/rio-rs/commit/85985a1ca111c2f46ad7ac37dc2dbb6f5d61815d) 

### 📚 Documentation

- Update internal docs [a95a866](https://github.com/rcelha/rio-rs/commit/a95a866126013ed7e916141aa9fa56418f3e9a38) 

### ⚙️ Miscellaneous Tasks

- Update roadmap [1842bd1](https://github.com/rcelha/rio-rs/commit/1842bd185b7ac2f8971f4e702f984a1e259526b0) 
- Break registry down into submodules [e612814](https://github.com/rcelha/rio-rs/commit/e612814dde7dd8453b81d1db4b9f2416a54ac9a9) 

## [0.2.3-alpha.1] - 2025-03-10

### 🚀 Features

- HttpMemberStorage [4d8d08b](https://github.com/rcelha/rio-rs/commit/4d8d08bae4c306730108c9457ca2b8e39a618126) 

### 🐛 Bug Fixes

- Improve error story [32b775c](https://github.com/rcelha/rio-rs/commit/32b775cd5d263815cb5acddb3c10f8d2ff120080) 

### 📚 Documentation

- Update roadmap and changelog [81d6ad6](https://github.com/rcelha/rio-rs/commit/81d6ad64dd0f07d30bc0f908ad1ea76d5f442787) 
- Add some badges [8d63327](https://github.com/rcelha/rio-rs/commit/8d63327f096fc20dbacca47a75d8e25f11b703e5) 
- Update roadmap [d918473](https://github.com/rcelha/rio-rs/commit/d9184739354823319fb240e4029a3bbe4da37732) 

## [0.2.1] - 2025-03-07

### 🚜 Refactor

- Add tools [1edfdf0](https://github.com/rcelha/rio-rs/commit/1edfdf045e4713f995237916e046da9f0f927f1c) 

## [0.2.0] - 2025-03-04

### 🚀 Features

- Implementation of naive server/client with clustering support (#3) [82a5d5a](https://github.com/rcelha/rio-rs/commit/82a5d5a71f230885cec3ae86315fc7dc537751ed) 
- *(state)* Add basic object state storage [ec490cd](https://github.com/rcelha/rio-rs/commit/ec490cdd0fb2f686e91ecc1911506f103437f4c7) 
- Add initial support to pub/sub (#10) [167a6c0](https://github.com/rcelha/rio-rs/commit/167a6c0d04db7e91338802266baf9be2673371b7) 
- Ephemeral ports and extended docs [73fd119](https://github.com/rcelha/rio-rs/commit/73fd1194588a6a636a0dc8c60d62db0221a7be85) 
- Extend supported backends - Redis, PgSQL (#13) [6db10d1](https://github.com/rcelha/rio-rs/commit/6db10d1498b41fd9d6b710492bcd84fb9014f048) 
- Expose client pool and internal fields for SendCommand [d2aa2b9](https://github.com/rcelha/rio-rs/commit/d2aa2b96f1995cbbb8cc0cf024b61505212debc2) 
- (BREAKING) Move the generic for StateLoader from the struct into the methods [3cea561](https://github.com/rcelha/rio-rs/commit/3cea5612414da3b98a0d2c658ff904a06928a098) 
- Custom error (#20) [e80dbb2](https://github.com/rcelha/rio-rs/commit/e80dbb240490e9e47456090e27511eb800220e77) 
- Split psql and sqlite support [08a5abd](https://github.com/rcelha/rio-rs/commit/08a5abd2afcbf84e274fb12302efdfa885222b74) 

### 🐛 Bug Fixes

- Membership sanitisation and Server aborting all tasks (#17) [520fd57](https://github.com/rcelha/rio-rs/commit/520fd57619689d82a7abc7f0537f631335caace4) 
- Redis cluster storage key mismatch on remove (#18) [588beb9](https://github.com/rcelha/rio-rs/commit/588beb9b34893b21cdb7d8ae499795290a791581) 
- Use the right version of async-trait [b5e7fca](https://github.com/rcelha/rio-rs/commit/b5e7fcabe657aee38b558b204eaccd746e91927c) 

### 💼 Other

- Add rust github workflow (#4) [3205090](https://github.com/rcelha/rio-rs/commit/3205090667ce4af381a0b7c530500cbd69a71cbb) 

### 🚜 Refactor

- Use tower (#5) [51e1060](https://github.com/rcelha/rio-rs/commit/51e106099438cb55196d96cdf7374b222c232460) 
- Remove boxed objects in serveral places (#6) [a8167e9](https://github.com/rcelha/rio-rs/commit/a8167e9e39b18e8f765925cc6af5fec185ba4489) 
- Remove FromId in favour of Default + WithId traits [1ebe7d3](https://github.com/rcelha/rio-rs/commit/1ebe7d386058d822331948b4cb1facfa59e2f2fb) 
- Replace registry (#14) [c513875](https://github.com/rcelha/rio-rs/commit/c51387565021543898a22413ed91ba41a63e64b0) 
- Transparent client for inside a service object [0e9fc29](https://github.com/rcelha/rio-rs/commit/0e9fc295453a032b49b5433046a956b58fdc7cd4) 
- (BREAKING) Spin up tasks from each server component in Server::run [bfdbb9f](https://github.com/rcelha/rio-rs/commit/bfdbb9f316595857de5adbc97e6f5d4bee883a99) 
- (BREAKING) Better separation between client and response error (#16) [0e20e20](https://github.com/rcelha/rio-rs/commit/0e20e20bad30cef46a305126a0bbdbb655d15941) 

### 📚 Documentation

- Update README.md [068f483](https://github.com/rcelha/rio-rs/commit/068f483ec7985c091963335c6777456fca690dd4) 
- Add a game server example (#9) [b2cf97e](https://github.com/rcelha/rio-rs/commit/b2cf97e4ed820decb6cfdee9cbf978bb88870e0d) 

### ⚙️ Miscellaneous Tasks

- Add license and note to work on COC [2add639](https://github.com/rcelha/rio-rs/commit/2add639af5bf98f29df33583471a69f61ff816ee) 
- Rename all the external APIs [64875b7](https://github.com/rcelha/rio-rs/commit/64875b72d62bba4ab34424c5bd46a5c2e7125ff5) 
- Fmt [7d0db9f](https://github.com/rcelha/rio-rs/commit/7d0db9f200a4b05283fdeac0c8891d06adcd7687) 
- Reorganize repository onto workspace (#11) [b1c97ed](https://github.com/rcelha/rio-rs/commit/b1c97edbcb2a4e545a4c5bfeeed43554351eec8d) 
- Start adding feature flags (#19) [3fe341c](https://github.com/rcelha/rio-rs/commit/3fe341c0cddef587a893a5f0442dca4b640a443c) 
- Update sqlx to v0.8 [b998848](https://github.com/rcelha/rio-rs/commit/b998848606c17792724ff150266478d0514f433b) 
- Clippy fixes [26a3634](https://github.com/rcelha/rio-rs/commit/26a363446e231283071b1dc2dde6cd9d1a7d857d) 

<!-- generated by git-cliff -->

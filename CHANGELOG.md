# Changelog

All notable changes to this project will be documented in this file.

## [unreleased]

### 📚 Documentation

- Update changelog [6371d9f](https://github.com/rcelha/rio-rs/commit/6371d9f03df6a64a1f455fd053cbc2e995ae4802) 
- Update README.md (#40) [58f4ea8](https://github.com/rcelha/rio-rs/commit/58f4ea894a0a4a6268cc268555fa0e9c491ccd3f) 

### ⚙️ Miscellaneous Tasks

- Move pre-release logic into the release.toml file [b316a7e](https://github.com/rcelha/rio-rs/commit/b316a7ecb9dd01e5d14bc593fa11b38a1e096d5d) 

## [0.5.1] - 2026-02-22

### 🚀 Features

- Use Builder as the public API to instantiate new Servers (#37) [d9d528b](https://github.com/rcelha/rio-rs/commit/d9d528b5a3f0f8b025001ef6a124ed7cb3c663c7) 
- Add tracing support and create an example integrated with Jaegger (#38) [54463c0](https://github.com/rcelha/rio-rs/commit/54463c064c2dbdb152448bc1d9bb56179f168588) 

### 🐛 Bug Fixes

- Remove redundant bounds from struct/trait declarations (#33) [904efec](https://github.com/rcelha/rio-rs/commit/904efec3d632e0148ff00cfb97e8f52a335b5467) 
- *(rio-macros)* Updated UI test expected output to match new compiler's format (#34) [1e8a542](https://github.com/rcelha/rio-rs/commit/1e8a5429826c6eb45b75d8aedeefd0d1401f3d24) 
- Argument on path for the http membership storage (server) [45c97ac](https://github.com/rcelha/rio-rs/commit/45c97ac33fe8146b64a2d677a3682a8353e881f1) 

### 🚜 Refactor

- Fix clippy warnings [f8854b1](https://github.com/rcelha/rio-rs/commit/f8854b1843ad0a27714fe02c343df6d74a50bf7e) 

### 📚 Documentation

- Flag netwatch for upgrade [96dd1b9](https://github.com/rcelha/rio-rs/commit/96dd1b949d95ae4aeb3839ce0cb03aa4bb706f91) 
- Update CHANGELOG.md [2edcd0b](https://github.com/rcelha/rio-rs/commit/2edcd0b8316abd6b8f98ec887a3e53de9975d893) 
- Update README.md [c8cbf04](https://github.com/rcelha/rio-rs/commit/c8cbf04f04d85d5a23ffd9d399aaf1e621e61498) 
- Improve make_registry docs and tests (#39) [5f4ae7d](https://github.com/rcelha/rio-rs/commit/5f4ae7d3a4da9c2a61d9b1ea531dd224c8ae83e3) 

### 🧪 Testing

- Improve move_object_on_server_failure to inspect the membership storage [0738dc1](https://github.com/rcelha/rio-rs/commit/0738dc18a6cff0efff4c7d923fed48fc00eb373f) 
- Improve comments on object_allocation integration test [6152e22](https://github.com/rcelha/rio-rs/commit/6152e22f1f49a7d7e71290ec94a309173abc5895) 
- Remove nextest from CI [1943c3e](https://github.com/rcelha/rio-rs/commit/1943c3e687c0fb53f863fbe18be65dc3b5b51f23) 

### ⚙️ Miscellaneous Tasks

- Remove async-recursion as it is not used anymore [7b63956](https://github.com/rcelha/rio-rs/commit/7b639562d61ded719799cb2658c288c89c2aa5f0) 
- Update async-stream [a64fd3e](https://github.com/rcelha/rio-rs/commit/a64fd3ea42289e5edb864f356b98215a91be0038) 
- Update axum [4820784](https://github.com/rcelha/rio-rs/commit/48207842ff378ed20cab06d3216949dc42ed7970) 
- Update bb8, bb8-redis, and redis [cf2bcc9](https://github.com/rcelha/rio-rs/commit/cf2bcc9dc3d7bc7730d86cea358f071a24489fc4) 
- Update chrono [2db7b30](https://github.com/rcelha/rio-rs/commit/2db7b3041a0509f539dae60239cb199568d011f9) 
- Upgrade dashmap [4c3b531](https://github.com/rcelha/rio-rs/commit/4c3b531098adbbdfc8878058f6c8cb2a680c49ef) 
- Upgrade derive_builder [3ba227b](https://github.com/rcelha/rio-rs/commit/3ba227bec1f5961c2f8e6f10167475fd8635e77c) 
- Upgrade env_logger [5f1b8f6](https://github.com/rcelha/rio-rs/commit/5f1b8f6b2d0c9265a03684f2fdd911c292157b7c) 
- Upgrade futures [ad24db1](https://github.com/rcelha/rio-rs/commit/ad24db186c9c2fffc093ce247f5c1b5faa3f4b83) 
- Upgrade log crate [267c685](https://github.com/rcelha/rio-rs/commit/267c68583d5e24d463e66105576c153ba005f3a7) 
- Update lru [2e8302e](https://github.com/rcelha/rio-rs/commit/2e8302ed14fed55f9c2d5be42899ff1cd0cccb5a) 
- Upgrade papaya [b61136b](https://github.com/rcelha/rio-rs/commit/b61136b35e9bd95e6f8f525e26bf3d4e288302ff) 
- Upgrade rand [cd5be76](https://github.com/rcelha/rio-rs/commit/cd5be76ccd04743f0aeee41978e36f877dc47b0c) 
- Upgrade reqwest [0e012a1](https://github.com/rcelha/rio-rs/commit/0e012a13a0f49a3f83fe22466e5f0c6a54233b21) 
- Relax versionin for state [b5809f4](https://github.com/rcelha/rio-rs/commit/b5809f46fff886d9f49e536204db5b237befe9c7) 
- Remove sync_wrapper as it is unused [41d271a](https://github.com/rcelha/rio-rs/commit/41d271afa8f5eb2d2c7f50e7d156aa7601bb258b) 
- Upgrade thiserror to 2 [9b0c48f](https://github.com/rcelha/rio-rs/commit/9b0c48f60d03eef4023302f9662ed417b6fe12c1) 
- Upgrade tokio-stream [3884a68](https://github.com/rcelha/rio-rs/commit/3884a68ee7d97e62e460931dbaa75adba0f18c16) 
- Upgrade tokio-util [5b39b09](https://github.com/rcelha/rio-rs/commit/5b39b098cd71a77226ab6f5c711974cb08ccdc73) 
- Upgrade tower [6e02f63](https://github.com/rcelha/rio-rs/commit/6e02f637fc635c3abfd914040416f626479e406d) 
- Update netwatch [333295e](https://github.com/rcelha/rio-rs/commit/333295e22dde2f19b0a2f3adb7a127e72d4c3757) 
- Reorganize metric-aggregator automation [234bdd2](https://github.com/rcelha/rio-rs/commit/234bdd20b3c1e0f7795e4b2d3e70fbcdca88472f) 

## [0.4.0] - 2025-04-04

### 🐛 Bug Fixes

- Improve error handling on pubsub (#32) [072259c](https://github.com/rcelha/rio-rs/commit/072259c7c92bafc0b99a95f1e4f823ddd3a4f1b8) 

## [0.3.3] - 2025-03-21

### 🚀 Features

- Backoff retry for client connection [cd9296f](https://github.com/rcelha/rio-rs/commit/cd9296f6a7aed71e5238e801fe0fca16d5930ad4) 
- Map local address when for client connection [223b740](https://github.com/rcelha/rio-rs/commit/223b7406689e20ead639485008101ff4be007eb4) 

### 💼 Other

- Run tests with nextest on CI (#30) [4ef4ab8](https://github.com/rcelha/rio-rs/commit/4ef4ab8ae3b476f58311c852f3d1bf399b91488d) 

### 📚 Documentation

- Update changelog format to include commit [df3745e](https://github.com/rcelha/rio-rs/commit/df3745ef2de01da2bcc6f240dd2fae1302d22dd5) 

### 🧪 Testing

- Fix failing ui test [a55128b](https://github.com/rcelha/rio-rs/commit/a55128b0fe38e52e994cbaee3c536745af2acc16) 

### ⚙️ Miscellaneous Tasks

- Update changelog [0f563b2](https://github.com/rcelha/rio-rs/commit/0f563b281b959c4fe80dc9d213210c125c1f7e5d) 
- Uses cache for CI [1eabd63](https://github.com/rcelha/rio-rs/commit/1eabd63a2d6f08d2ed9b9d64122c3d68c3d75019) 
- Generate only this crates docs [c8c52c3](https://github.com/rcelha/rio-rs/commit/c8c52c3d5b46537d06eb89773464cc530b3583f4) 

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

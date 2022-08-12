#![allow(unused)]

use criterion::async_executor::*;
use criterion::BenchmarkId;

use serde::Deserialize;
use serde::Serialize;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::RwLock;

use rio_rs::prelude::*;
use rio_rs::registry::IdentifiableType;

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize, Message, TypeName)]
struct Ping {}

#[derive(Default, Debug, PartialEq, Serialize, Deserialize, Message, TypeName)]
struct Pong {}

#[derive(Default, Debug, PartialEq, Serialize, Deserialize, Message, TypeName)]
struct Bounce {}

#[derive(Default, Debug, TypeName)]
struct Ball {
    pub kind: &'static str,
}

#[async_trait::async_trait]
impl Handler<Ping> for Ball {
    type Returns = Pong;

    async fn handle(&mut self, _: Ping, _: Arc<AppData>) -> Result<Self::Returns, HandlerError> {
        Ok(Pong {})
    }
}

#[async_trait::async_trait]
impl Handler<Bounce> for Ball {
    type Returns = Bounce;

    async fn handle(
        &mut self,
        message: Bounce,
        _: Arc<AppData>,
    ) -> Result<Self::Returns, HandlerError> {
        Ok(message)
    }
}

async fn channel_registry_handle(
    options: &(&mut rio_rs::channel_registry::Registry, Arc<AppData>, usize),
) {
    let (r, app_data, n) = options;
    for i in 1..*n {
        let response = r
            .send::<Ball, Ping, Pong>("round", Ping {}, app_data.clone())
            .await
            .unwrap();
        assert_eq!(response, Pong {});
    }
}

async fn registry_handle(options: &(&mut Registry, Arc<AppData>, usize)) {
    let (r, app_data, n) = options;
    for i in 1..*n {
        let response = r
            .send(
                &Ball::user_defined_type_id(),
                "round",
                &Ping::user_defined_type_id(),
                &bincode::serialize(&Ping {}).unwrap(),
                app_data.clone(),
            )
            .await
            .unwrap();
        let response: Pong = bincode::deserialize(&response).unwrap();
        assert_eq!(response, Pong {});
    }
}

use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn registry_benchmark_channel_registry(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let (mut r, app_data) = runtime.block_on(async {
        let mut r = rio_rs::channel_registry::Registry::new();
        r.register::<Ball, Ping, Pong>().unwrap();
        r.register::<Ball, Bounce, Bounce>().unwrap();

        let object = Arc::new(RwLock::new(Ball { kind: "round" }));
        let object2 = Arc::new(RwLock::new(Ball { kind: "square" }));

        r.add_object::<_, Ping, Pong>(object.clone(), "round".to_string())
            .unwrap();
        r.add_object::<_, Ping, Pong>(object2.clone(), "square".to_string())
            .unwrap();
        r.add_object::<_, Bounce, Bounce>(object.clone(), "round".to_string())
            .unwrap();

        let app_data = Arc::new(AppData::new());
        (r, app_data)
    });

    for size in [1, 100, 10_000] {
        let mut options = (&mut r, app_data.clone(), size);
        c.bench_with_input(
            BenchmarkId::new("channel_registry handle", size),
            &options,
            |b, s| {
                b.to_async(&runtime).iter(|| channel_registry_handle(s));
            },
        );
    }
}

fn registry_benchmark_registry(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let (mut r, app_data) = runtime.block_on(async {
        let mut r = Registry::new();
        r.add_handler::<Ball, Ping>();
        r.add_handler::<Ball, Bounce>();

        let object = Ball { kind: "round" };
        let object2 = Ball { kind: "square" };

        r.add("round".into(), object).await;
        r.add("square".into(), object2).await;

        let app_data = Arc::new(AppData::new());

        (r, app_data)
    });

    for size in [1, 100, 10_000] {
        {
            let options = &(&mut r, app_data.clone(), size);
            c.bench_with_input(
                BenchmarkId::new("registry handle", size),
                &options,
                |b, s| {
                    b.to_async(&runtime).iter(|| registry_handle(s));
                },
            );
        }
    }
}

pub fn registry_benchmark(c: &mut Criterion) {
    registry_benchmark_registry(c);
    registry_benchmark_channel_registry(c);
}

criterion_group!(benches, registry_benchmark);
criterion_main!(benches);

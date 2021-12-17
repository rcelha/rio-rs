use bb8::Pool;
use futures::sink::SinkExt;
use std::sync::Arc;
use tokio::io::BufReader;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tokio_stream::StreamExt;
use tokio_util::codec::{Framed, LengthDelimitedCodec};

use crate::app_data::AppData;
use crate::client::ClientConnectionManager;
use crate::cluster_provider::ClusterProvider;
use crate::grain_placement_provider::{GrainPlacement, GrainPlacementProvider};
use crate::membership_provider::MembersStorage;
use crate::protocol::{RequestEnvelope, ResponseEnvelope, ResponseError};
use crate::registry::Registry;
use crate::{GrainId, LifecycleMessage};

pub struct Silo<T>
where
    T: MembersStorage + 'static,
{
    address: String,
    registry: Arc<RwLock<Registry>>,
    membership_provider: Box<dyn ClusterProvider<T>>,
    grain_placement_provider: Arc<RwLock<dyn GrainPlacementProvider>>,
    app_data: Arc<AppData>,
}

impl<T> Silo<T>
where
    T: MembersStorage,
{
    pub fn new(
        address: String,
        registry: Registry,
        membership_provider: impl ClusterProvider<T> + 'static,
        grain_placement_provider: impl GrainPlacementProvider + 'static,
    ) -> Silo<T> {
        Silo {
            address,
            registry: Arc::new(RwLock::new(registry)),
            membership_provider: Box::new(membership_provider),
            grain_placement_provider: Arc::new(RwLock::new(grain_placement_provider)),
            app_data: Arc::new(AppData::new()),
        }
    }

    pub fn app_data<Data>(&mut self, data: Data)
    where
        Data: Send + Sync + 'static,
    {
        self.app_data.set(data);
    }

    async fn silo_serve(&self) {
        let listener = TcpListener::bind(&self.address)
            .await
            .expect("TODO: Failed to bind address");
        println!("Listening on: {}", self.address);
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let handler: SiloClientHandler = self.into();
            tokio::spawn(async move { handler.handle_client(stream).await });
        }
    }

    // TODO make client pool configurable
    pub async fn serve(&mut self) {
        let boxed_storage = dyn_clone::clone_box(self.membership_provider.members_storage());
        let pool_manager = ClientConnectionManager::new(boxed_storage);
        let client_pool = Pool::builder()
            .max_size(10)
            .build(pool_manager)
            .await
            .unwrap();
        self.app_data(client_pool);

        tokio::select! {
            _ = self.silo_serve() => {
                println!("serve finished first");
            }
            _ = self.membership_provider.serve(&self.address)  => {
                println!("serve finished first");
            }
        };
    }
}

struct SiloClientHandler {
    address: String,
    registry: Arc<RwLock<Registry>>,
    members_storage: Box<dyn MembersStorage>,
    grain_placement_provider: Arc<RwLock<dyn GrainPlacementProvider>>,
    app_data: Arc<AppData>,
}

impl<T> From<&Silo<T>> for SiloClientHandler
where
    T: MembersStorage,
{
    fn from(silo: &Silo<T>) -> Self {
        let address = silo.address.clone();
        let registry = silo.registry.clone();
        let grain_placement_provider = silo.grain_placement_provider.clone();
        let app_data = silo.app_data.clone();
        let members_storage = dyn_clone::clone_box(silo.membership_provider.members_storage());

        SiloClientHandler {
            address,
            registry,
            members_storage,
            grain_placement_provider,
            app_data,
        }
    }
}

impl SiloClientHandler {
    // TODO move upsert to storage
    async fn upsert_placement(&self, handler_type: String, handler_id: String) -> String {
        if self
            .registry
            .read()
            .await
            .has(&handler_type, &handler_id)
            .await
        {
            return self.address.clone();
        }

        let grain_id = GrainId::new(handler_type, handler_id);
        let mut maybe_silo_address = self
            .grain_placement_provider
            .read()
            .await
            .lookup(&grain_id)
            .await;

        if maybe_silo_address.is_none() {
            let new_placement = GrainPlacement::new(grain_id, Some(self.address.clone())); // TODO: choose one at random
            let new_address = new_placement.silo_address.clone().unwrap();
            self.grain_placement_provider
                .write()
                .await
                .update(new_placement)
                .await;
            maybe_silo_address = Some(new_address)
        }
        maybe_silo_address.unwrap()
    }

    // TODO tune LightDelimitedCodec
    async fn handle_client(&self, stream: TcpStream) {
        let stream = BufReader::new(stream);
        let codec = LengthDelimitedCodec::new();
        let mut frames = Framed::new(stream, codec);

        while let Some(Ok(frame)) = frames.next().await {
            let request_envelope: RequestEnvelope = match bincode::deserialize(&frame) {
                Ok(v) => v,
                Err(e) => {
                    panic!("Failed to unpack {:?} -> {:?}", frame, e);
                }
            };
            let silo_address = self
                .upsert_placement(
                    request_envelope.handler_type.clone(),
                    request_envelope.handler_id.clone(),
                )
                .await;

            if silo_address != self.address {
                // TODO error handling
                let mut split_address = silo_address.split(':');
                let ip = split_address.next().expect("TODO: Address has no IP in it");
                let port = split_address
                    .next()
                    .expect("TODO: Address has no PORT in it");

                // TODO cache is_active response?
                let error = if self
                    .members_storage
                    .is_active(ip, port)
                    .await
                    .expect("TODO")
                {
                    ResponseError::Redirect(silo_address)
                } else {
                    self.grain_placement_provider
                        .read()
                        .await
                        .clean_silo(silo_address)
                        .await;
                    ResponseError::DeallocateGrain
                };
                let response_envelope = ResponseEnvelope::err(error);
                let ser_response = bincode::serialize(&response_envelope).unwrap();
                frames.send(ser_response.into()).await.unwrap();
                continue;
            }

            if !self
                .registry
                .read()
                .await
                .has(
                    &request_envelope.handler_type.clone(),
                    &request_envelope.handler_id.clone(),
                )
                .await
            {
                let new_object = self
                    .registry
                    .read()
                    .await
                    .call_static_fn_box(
                        request_envelope.handler_type.clone(),
                        request_envelope.handler_id.clone(),
                    )
                    .unwrap();

                self.registry
                    .read()
                    .await
                    .insert_boxed_object(
                        request_envelope.handler_type.clone(),
                        request_envelope.handler_id.clone(),
                        new_object,
                    )
                    .await;

                let _ = self
                    .registry
                    .read()
                    .await
                    .send(
                        &request_envelope.handler_type,
                        &request_envelope.handler_id,
                        "LifecycleMessage",
                        &bincode::serialize(&LifecycleMessage::Load).unwrap(),
                        self.app_data.clone(),
                    )
                    .await;
            }

            let response = self
                .registry
                .read()
                .await
                .send(
                    &request_envelope.handler_type,
                    &request_envelope.handler_id,
                    &request_envelope.message_type,
                    &request_envelope.payload,
                    self.app_data.clone(),
                )
                .await;
            let response_envelope = match response {
                Ok(body) => ResponseEnvelope::new(body),
                Err(err) => ResponseEnvelope::from(err),
            };
            let ser_response = bincode::serialize(&response_envelope).unwrap();
            frames.send(ser_response.into()).await.unwrap();
        }
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_sanity() {
        // todo!("Add tests for silos");
    }
}

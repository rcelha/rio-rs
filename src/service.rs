use futures::future::BoxFuture;
use futures::sink::SinkExt;
use pin_project::pin_project;
use std::sync::Arc;
use tokio::io::BufReader;
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio_stream::StreamExt;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tower::Service as TowerService;

use crate::app_data::AppData;
use crate::cluster::storage::MembersStorage;
use crate::object_placement::ObjectPlacementProvider;
use crate::protocol::{RequestEnvelope, ResponseEnvelope, ResponseError};
use crate::registry::Registry;
use crate::{LifecycleMessage, ObjectId};

#[pin_project]
pub struct Service {
    #[pin]
    pub(crate) address: String,
    pub(crate) registry: Arc<RwLock<Registry>>,
    pub(crate) members_storage: Box<dyn MembersStorage>,
    pub(crate) object_placement_provider: Arc<RwLock<dyn ObjectPlacementProvider>>,
    pub(crate) app_data: Arc<AppData>,
}

impl TowerService<RequestEnvelope> for Service {
    type Response = ResponseEnvelope;
    type Error = ResponseError;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: RequestEnvelope) -> Self::Future {
        let address = self.address.clone();
        let members_storage = self.members_storage.clone();
        let object_placement_provider = self.object_placement_provider.clone();
        let registry = self.registry.clone();
        let app_data = self.app_data.clone();

        let result = async move {
            let silo_address = Self::upsert_placement(
                object_placement_provider.clone(),
                address.clone(),
                req.handler_type.clone(),
                req.handler_id.clone(),
            )
            .await;

            if silo_address != address {
                // TODO error handling
                let mut split_address = silo_address.split(':');
                let ip = split_address.next().expect("TODO: Address has no IP in it");
                let port = split_address
                    .next()
                    .expect("TODO: Address has no PORT in it");

                // TODO cache is_active response?
                let error = if members_storage.is_active(ip, port).await.expect("TODO") {
                    ResponseError::Redirect(silo_address)
                } else {
                    object_placement_provider
                        .read()
                        .await
                        .clean_silo(silo_address)
                        .await;
                    ResponseError::DeallocateServiceObject
                };
                return Err(error);
            }

            if !registry
                .read()
                .await
                .has(&req.handler_type.clone(), &req.handler_id.clone())
                .await
            {
                let new_object = registry
                    .read()
                    .await
                    .call_static_fn_box(req.handler_type.clone(), req.handler_id.clone())
                    .unwrap();

                registry
                    .read()
                    .await
                    .insert_boxed_object(
                        req.handler_type.clone(),
                        req.handler_id.clone(),
                        new_object,
                    )
                    .await;

                let _ = registry
                    .read()
                    .await
                    .send(
                        &req.handler_type,
                        &req.handler_id,
                        "LifecycleMessage",
                        &bincode::serialize(&LifecycleMessage::Load).unwrap(),
                        app_data.clone(),
                    )
                    .await;
            }

            let response = registry
                .read()
                .await
                .send(
                    &req.handler_type,
                    &req.handler_id,
                    &req.message_type,
                    &req.payload,
                    app_data.clone(),
                )
                .await;
            match response {
                Ok(body) => Ok(ResponseEnvelope::new(body)),
                Err(err) => Err(ResponseError::Unknown(format!(
                    "[TODO] HandlerError: {}",
                    err
                ))),
            }
        };
        Box::pin(result)
    }
}

impl Service {
    async fn upsert_placement(
        object_placement_provider: Arc<RwLock<dyn ObjectPlacementProvider>>,
        address: String,
        handler_type: String,
        handler_id: String,
    ) -> String {
        object_placement_provider
            .write()
            .await
            .upsert(ObjectId(handler_type, handler_id), address)
            .await
    }

    // TODO tune LightDelimitedCodec
    // TODO move this into a transport struct
    pub async fn run(&mut self, stream: TcpStream) {
        let stream = BufReader::new(stream);
        let codec = LengthDelimitedCodec::new();
        let mut frames = Framed::new(stream, codec);

        while let Some(Ok(frame)) = frames.next().await {
            let request: RequestEnvelope = match bincode::deserialize(&frame) {
                Ok(v) => v,
                Err(e) => {
                    panic!("TODO Failed to unpack {:?} -> {:?}", frame, e);
                }
            };
            let response = match self.call(request).await {
                Ok(x) => x,
                Err(err) => ResponseEnvelope::err(err),
            };
            let ser_response = bincode::serialize(&response).expect("TODO");
            frames.send(ser_response.into()).await.unwrap();
        }
    }
}

#[cfg(test)]
mod test {
    use tower::ServiceExt;

    use super::*;
    use crate::cluster::storage::LocalStorage;
    use crate::object_placement::local::LocalObjectPlacementProvider;

    fn svc() -> Service {
        Service {
            address: "0.0.0.0:5000".to_string(),
            registry: Arc::new(RwLock::new(Registry::new())),
            members_storage: Box::new(LocalStorage::default()),
            object_placement_provider: Arc::new(RwLock::new(
                LocalObjectPlacementProvider::default(),
            )),
            app_data: Arc::new(AppData::new()),
        }
    }

    #[tokio::test]
    async fn test_poll_ready() {
        let mut svc = svc();
        svc.ready().await.expect("service ready");
    }
}

use anyhow::Result;
use exports::rio_rs::services::people;
use exports::rio_rs::services::people_ping;
use exports::rio_rs::services::people_pong;
use wasmtime::component::{Component, Linker, ResourceTable, bindgen};
use wasmtime::*;
use wasmtime_wasi::{IoView, WasiCtx, WasiCtxBuilder, WasiView};

pub struct ComponentRunStates {
    pub wasi_ctx: WasiCtx,
    pub resource_table: ResourceTable,
}

impl IoView for ComponentRunStates {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.resource_table
    }
}

impl WasiView for ComponentRunStates {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi_ctx
    }
}

bindgen!(in "wit/rio-rs/");

fn main() -> Result<()> {
    let eng = Engine::default();
    let mut linker = Linker::new(&eng);
    wasmtime_wasi::add_to_linker_sync(&mut linker)?;
    // wasmtime_wasi::add_to_linker_async(&mut linker)?;

    let wasi_ctx = WasiCtxBuilder::new().inherit_stdio().build();

    let store_data = ComponentRunStates {
        resource_table: ResourceTable::default(),
        wasi_ctx,
    };
    let mut store = Store::new(&eng, store_data);

    println!("loading module");
    let component = Component::from_file(&eng, "app.wasm")?;

    let bindings = RioService::instantiate(&mut store, &component, &linker)?;
    let people_ping_bind = bindings.rio_rs_services_people_ping();
    let service_bind = bindings.rio_rs_services_people().service();

    let service = service_bind.call_constructor(&mut store, "1")?;
    println!("svc #1 {:?}", service);
    let message = people_ping::Ping { count: 1 };
    println!("msg #1 {:?}", message);
    let resp = people_ping_bind.call_handle(&mut store, service, message)?;
    println!("resp #1 {:?}", resp);

    let bindings = RioService::instantiate(&mut store, &component, &linker)?;
    let people_ping_bind = bindings.rio_rs_services_people_ping();
    let service_bind = bindings.rio_rs_services_people().service();

    let service = service_bind.call_constructor(&mut store, "666")?;
    println!("svc #2 {:?}", service);
    let message = people_ping::Ping { count: 10 };
    println!("msg #2 {:?}", message);
    let resp = people_ping_bind.call_handle(&mut store, service, message);
    println!("resp #2 {:?}", resp);

    Ok(())
}

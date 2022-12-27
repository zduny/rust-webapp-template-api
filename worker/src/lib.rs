use wasm_bindgen::prelude::*;
use js_utils::{console_log, set_panic_hook};
use kodec::binary::Codec;
use mezzenger_webworker::Transport;
use common::api::worker::Producer;

use zzrpc::producer::{Produce, Configuration};

#[wasm_bindgen(start)]
pub async fn main_worker() -> Result<(), JsValue> {
    set_panic_hook();

    console_log!("Worker: worker started!");
    let transport =
        Transport::new_in_worker(Codec::default())
            .await
            .unwrap();
    let _producer = Producer {}.produce(transport, Configuration::default());
    console_log!("Worker: transport open and producing.");

    Ok(())
}

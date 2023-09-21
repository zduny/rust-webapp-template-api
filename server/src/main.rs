mod state;

use std::{env::current_dir, sync::Arc};

use anyhow::Result;
use futures::Stream;
use kodec::binary::Codec;
use mezzenger_websocket::warp::Transport;
use tokio::{signal::ctrl_c, spawn, sync::RwLock};
use tokio_stream::{wrappers::BroadcastStream, StreamExt};
use tracing::{error, info, Level};
use warp::{
    hyper::StatusCode,
    ws::{WebSocket, Ws},
    Filter,
};

type State = Arc<RwLock<state::State>>;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("Server running!");

    let current_dir = current_dir()?;
    info!("Current working directory: {:?}.", current_dir);

    let state = Arc::new(RwLock::new(state::State::new()));
    let state = warp::any().map(move || state.clone());
    let websocket = warp::path("ws")
        .and(warp::ws())
        .and(state)
        .map(|ws: Ws, state| ws.on_upgrade(move |web_socket| user_connected(web_socket, state)));

    let static_files = warp::get().and(warp::fs::dir("www"));
    let routes = websocket.or(static_files).recover(handle_rejection);

    let (address, server_future) =
        warp::serve(routes).bind_with_graceful_shutdown(([127, 0, 0, 1], 8080), async move {
            ctrl_c()
                .await
                .expect("unable to listen for shutdown signal");
        });
    let server_handle = spawn(server_future);
    info!("Listening at {}...", address);

    server_handle.await?;
    info!("Shutting down...");

    Ok(())
}

use common::api::chat::*;
use zzrpc::{
    producer::{Configuration, Produce},
    Produce,
};

#[derive(Produce)]
struct Producer {
    state: State,
    user_name: String,
}

impl Producer {
    /// Get user name.
    async fn user_name(&self) -> String {
        self.user_name.clone()
    }

    /// Get other connected user names.
    async fn user_names(&self) -> Vec<String> {
        let my_name = self.user_name.clone();
        self.state
            .read()
            .await
            .users
            .values()
            .map(|user| &user.name)
            .filter(move |name| name != &&my_name)
            .cloned()
            .collect()
    }

    /// Send chat message.
    async fn message(&self, message: String) {
        let _ = self
            .state
            .read()
            .await
            .message_sender
            .send((self.user_name.clone(), message));
    }

    /// Stream of pairs containing: (user name, message)
    async fn messages(&self) -> impl Stream<Item = (String, String)> {
        BroadcastStream::new(self.state.read().await.message_sender.subscribe())
            .filter_map(Result::ok)
    }

    /// Stream of names of newly connected users.
    async fn connected(&self) -> impl Stream<Item = String> {
        let my_name = self.user_name.clone();
        BroadcastStream::new(self.state.read().await.connected_sender.subscribe())
            .filter_map(Result::ok)
            .filter(move |name| name != &my_name)
    }

    /// Stream of names of disconnected users.
    async fn disconnected(&self) -> impl Stream<Item = String> {
        let my_name = self.user_name.clone();
        BroadcastStream::new(self.state.read().await.disconnected_sender.subscribe())
            .filter_map(Result::ok)
            .filter(move |name| name != &my_name)
    }
}

async fn user_connected(web_socket: WebSocket, state: State) {
    let codec = Codec::default();
    let transport = Transport::new(web_socket, codec);
    let (id, name) = {
        let mut state_lock = state.write().await;
        let user = state_lock.add_user();
        (user.id, user.name.clone())
    };
    info!("User <{name}> connected.");
    let producer = Producer {
        state: state.clone(),
        user_name: name.clone(),
    };
    producer
        .produce(transport, Configuration::default())
        .await
        .unwrap();

    user_disconnected(id, name, &state).await;
}

async fn user_disconnected(id: usize, name: String, state: &State) {
    info!("User <{name}> disconnected.");
    let mut state = state.write().await;
    state.users.remove(&id);
    let _ = state.disconnected_sender.send(name);
}

async fn handle_rejection(
    err: warp::Rejection,
) -> Result<impl warp::Reply, std::convert::Infallible> {
    if err.is_not_found() {
        error!("Error occurred: {:?}.", err);
        Ok(warp::reply::with_status("Not found", StatusCode::NOT_FOUND))
    } else {
        error!("Error occurred: {:?}.", err);
        Ok(warp::reply::with_status(
            "Internal server error",
            StatusCode::INTERNAL_SERVER_ERROR,
        ))
    }
}

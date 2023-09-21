use std::{net::SocketAddr, time::Duration, collections::HashMap, sync::Arc};
use axum::{
    routing::get,
    Router,
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, ConnectInfo, State},
    response::{IntoResponse, Html}, TypedHeader, headers, http::HeaderValue,
};
use shared::*;
use tokio::{sync::{mpsc::{Receiver, self, Sender}, oneshot, RwLock}, time::Instant};
//allows to split the websocket stream into separate TX and RX branches
use futures::{sink::SinkExt, stream::StreamExt};

#[tokio::main]
async fn main() {
    let (tx, rx) = mpsc::channel::<GameStateMsg>(32);
    // powers the game's "central clock"
    // the game_state_manager receives a tick on a fixed schedule
    // and it performs calculations, broadcasts.
    let tx2 = tx.clone();
    tokio::spawn(async move {
        let time_to_sleep = (TIME_PER_TICK * 100) as u64;
        loop {
            // println!("LOOP1!");
            tokio::time::sleep(Duration::from_micros(time_to_sleep)).await;
            // println!("LOOP2!");
            let _ = tx2.send(GameStateMsg::Tick).await;
        }
    });
    let state = Arc::new(RwLock::new(AppState::new(tx)));
    tokio::spawn(async move {
        game_state_manager(rx).await;
    });

    let app = Router::new()
        .route("/", get(homepage))
        .route("/index.html", get(homepage))
        .route("/game.wasm", get(wasmbinary))
        .route("/ws", get(ws_handler).with_state(state));

    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();
}

type SharedState = Arc<RwLock<AppState>>;

struct AppState {
    game_state_tx: Sender<GameStateMsg>,
    pub id_counter: u64,
}
impl AppState {
    pub fn new(tx: Sender<GameStateMsg>) -> Self {
        Self { game_state_tx: tx, id_counter: 0 }
    }
}

pub enum GameStateMsg {
    PlayerRegistered { id: u64, tx: Sender<Arc<GameOutputMessage>> },
    MoveBy { id: u64, mx: f32, my: f32 },
    Tick,
}


#[derive(Clone)]
pub struct PlayerState {
    pub x: f32,
    pub y: f32,
    pub last_cmd_at: Instant,
    pub tx: Sender<Arc<GameOutputMessage>>,
}

#[derive(Default, Clone)]
pub struct GameState {
    pub players: HashMap<u64, PlayerState>,
}

impl GameState {
    pub fn to_output_msg(&self) -> GameOutputMessage {
        let mut positions = HashMap::new();
        for (id, player_state) in self.players.iter() {
            positions.insert(*id, (player_state.x, player_state.y));
        }
        GameOutputMessage::PlayerPositions { positions }
    }
}

async fn game_state_manager(mut rx: Receiver<GameStateMsg>) {
    let mut game_state =  GameState::default(); 
    while let Some(cmd) = rx.recv().await {
        match cmd {
            GameStateMsg::Tick => {
                // println!("TICK");
                let out_msg = game_state.to_output_msg();
                let out = Arc::new(out_msg);
                let mut removes = vec![];
                for (id, player) in game_state.players.iter() {
                    if let Err(_e) = player.tx.send(out.clone()).await {
                        println!("Player {} has disconnected. removing them from future tick updates", id);
                        removes.push(*id);
                    }
                }
                for id in removes {
                    game_state.players.remove(&id);
                }
            }
            GameStateMsg::PlayerRegistered { id, tx } => {
                let out = Arc::new(GameOutputMessage::YouAre { id });
                if let Err(e) = tx.send(out).await {
                    println!("Failed to give player initial id={} {:?}", id, e);
                }
                game_state.players.insert(id, PlayerState {
                    x: START_X_Y, y: START_X_Y, tx,
                    last_cmd_at: Instant::now(),
                });
            }
            GameStateMsg::MoveBy { id, mx, my } => {
                let player = match game_state.players.get_mut(&id) {
                    Some(p) => p,
                    None => {
                        println!("No player {id} found");
                        continue
                    },
                };
                let now = tokio::time::Instant::now();
                let time_delta = now.duration_since(player.last_cmd_at).as_millis();
                let time_delta = time_delta * 10;
                // prevent client code from sending more often than the tick rate
                if time_delta < TIME_PER_TICK {
                    println!("Player {id} is sending too fast. Last message was {}ms ago!", time_delta / 10);
                    continue;
                }
                player.last_cmd_at = now;
                // prevent cheating: server should know the maximum movement
                // a player can make in a given time span
                if mx.abs() > MAX_MOVEMENT || my.abs() > MAX_MOVEMENT {
                    println!("Player {id} is suspected of cheating. Movement was greater than {MAX_MOVEMENT}");
                    continue;
                }
                player.x += mx;
                player.y += my;
                fix_position_within_bounds(&mut player.x, &mut player.y);
                println!("Player {id} is now at {},{}", player.x, player.y);
            }
        }
    }
}

async fn homepage() -> impl IntoResponse {
    Html(include_str!("../assets/index.html")).into_response()
}

async fn wasmbinary() -> impl IntoResponse {
    let mut resp = include_bytes!("../../target/wasm32-unknown-unknown/release/client.wasm").into_response();
    resp.headers_mut().insert("Content-Type", HeaderValue::from_static("application/wasm"));
    resp
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    // user_agent: Option<TypedHeader<headers::UserAgent>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<SharedState>,
) -> impl IntoResponse {
    // let user_agent = if let Some(TypedHeader(user_agent)) = user_agent {
    //     user_agent.to_string()
    // } else {
    //     String::from("Unknown browser")
    // };
    // // finalize the upgrade process by returning upgrade callback.
    // // we can customize the callback by sending additional info such as address.
    ws.on_upgrade(move |socket| handle_socket(socket, addr, state.clone()))
}

/// Actual websocket statemachine (one will be spawned per connection)
async fn handle_socket(socket: WebSocket, who: SocketAddr, state: SharedState) {
    let (gameout_tx, mut gameout_rx) = mpsc::channel::<Arc<GameOutputMessage>>(32);
    let id = {
        let mut w = state.write().await;
        let prev_id = w.id_counter;
        w.id_counter += 1;
        let _ = w.game_state_tx.send(GameStateMsg::PlayerRegistered { id: prev_id, tx: gameout_tx }).await;
        prev_id
    };
    println!("{who} connected. Id {id}");
    let (mut sender, mut receiver) = socket.split();

    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = gameout_rx.recv().await {
            if let Err(e) = sender.send(Message::Text(msg.serialize_json())).await {
                println!("Error sending message for player {id}. {e}. closing their connection");
                break;
            }
        }
    });
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Close(_) = msg {
                break;
            }
            let msg_txt = if let Message::Text(txt) = msg {
                txt
            } else { continue; };
            // failure to deserialize means user is on an invalid client, drop em
            let ws_msg: GameInputMessage = if let Ok(deserd) = serde_json::from_str(&msg_txt) {
                deserd
            } else { break; };
            match ws_msg {
                GameInputMessage::Move { mx, my } => {
                    let r = state.read().await;
                    let _ = r.game_state_tx.send(GameStateMsg::MoveBy { id, mx, my }).await;
                }
            }
        }
    });

    // If any one of the tasks exit, abort the other.
    tokio::select! {
        _ = (&mut send_task) => {
            println!("SEND TASK ENDED");
            recv_task.abort();
        },
        _ = (&mut recv_task) => {
            println!("RX TASK ENDED");
            send_task.abort();
        }
    }
    println!("{id} disconnected");
}

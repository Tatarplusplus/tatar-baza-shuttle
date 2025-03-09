use axum::{extract::State, response::Html, routing::any, Router};
use tokio::{net::TcpListener, sync::broadcast};
use std::{include_str, sync::Arc};   
use std::collections::HashSet;
use std::sync::Mutex;
use tokio;
use axum::response::IntoResponse;
use axum::extract::ws::{Message, Utf8Bytes, WebSocket, WebSocketUpgrade};
use futures::{sink::SinkExt, stream::StreamExt};
// post locally, get only public

struct AppState {
    user_set: Mutex<HashSet<String>>,
    tx: broadcast::Sender<String>,
}

#[shuttle_runtime::main]
async fn main() -> shuttle_axum::ShuttleAxum {
    let authors_routes = Router::new()
        .route("/", any(authors))
        .route("/arsen_markaryan", any(arsen))
        .route("/mikhail_svetov", any(svetov));

    let chat_routes = Router::new()
        .route("/", any(chat_main))
        .route("/websocket", any(chat_handler));

    let user_set = Mutex::new(HashSet::new());
    let (tx, _) = broadcast::channel(100);

    let app_state = Arc::new(AppState { user_set, tx });

    let app = Router::new()
        .route("/", any(root))
        .nest("/chat", chat_routes)
        .nest("/authors", authors_routes)
        .fallback(fallback)
        .with_state(app_state);

    //let listener = TcpListener::bind("0.0.0.0:10000").await.unwrap();
    //axum::serve(listener, app).await.unwrap();
    Ok(app.into())
}

async fn chat_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse{
    ws.on_upgrade(|socket| chat(socket, state))
}

async fn chat(stream: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = stream.split();
    let mut username = String::new();

    while let Some(Ok(Message::Text(name))) = receiver.next().await {

        check_username(&state, &mut username, name.as_str());

        if !name.is_empty() {
            break;
        } else {
            let _ = sender
                .send(Message::Text(Utf8Bytes::from_static(
                    "Username already exists"
                )))
                .await;

            return;
        }
    }

    let mut rx = state.tx.subscribe();

    let message = format!("{username} joined.");
    let _ = state.tx.send(message);

    let mut send_task = tokio::spawn(async move {
        while let Ok(message) = rx.recv().await {
            if sender.send(Message::text(message)).await.is_err() {
                break;
            }
        }
    });

    let tx = state.tx.clone();
    let name = username.clone();

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(message))) = receiver.next().await {
            let _ = tx.send(format!("{username}: {message}"));
        }
    });

    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    };

    let message = format!("{name} left.");
    let _ = state.tx.send(message);

    state.user_set.lock().unwrap().remove(&name);
}

fn check_username(state: &AppState, string: &mut String, name: &str) {
    let mut user_set = state.user_set.lock().unwrap();

    if !user_set.contains(name) {
        user_set.insert(name.to_owned());
        
        string.push_str(name);
    }
}

async fn root() -> Html<&'static str> {
    Html(include_str!("../htmls/root.html"))
}

async fn chat_main() -> Html<&'static str> {
    Html(include_str!("../htmls/chat.html"))
}

async fn authors() -> Html<&'static str> {
    Html(include_str!("../htmls/authors.html"))
} 

async fn arsen() -> Html<&'static str> {
    Html(include_str!("../htmls/arsen.html"))
}

async fn svetov() -> Html<&'static str> {
    Html(include_str!("../htmls/svetov.html"))
}

async fn fallback() -> Html<&'static str> {
    Html(include_str!("../htmls/fallback.html"))
}
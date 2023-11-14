use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use serde_json::{json, Value};
use std::{net::SocketAddr, sync::Arc};
use todo_logic::{Pagination, TodoItem, TodoStore, TodoStoreError, UpdateTodoItem};
use tokio::sync::RwLock;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utils::update_resp;

/// Type for our shared state
///
/// In our sample application, we store the todo list in memory. As the state is shared
/// between concurrently running web requests, we need to make it thread-safe.
type Db = Arc<RwLock<TodoStore>>;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    // let rust_log = std::env::var("RUST_LOG").expect("asgfiuagsuif");
    // tracing::debug!(rust_log);
    // Enable tracing using Tokio's https://tokio.rs/#tk-lib-tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "axu=debug,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Create shared data store
    let db = Db::default();
    // let db_mysql = PgPoolOptions::new()
    //     // The default connection limit for a Postgres server is 100 connections, minus 3 for superusers.
    //     // Since we're using the default superuser we don't have to worry about this too much,
    //     // although we should leave some connections available for manual access.
    //     //
    //     // If you're deploying your application with multiple replicas, then the total
    //     // across all replicas should not exceed the Postgres connection limit.
    //     .max_connections(50)
    //     .connect(&config.database_url)
    //     .await
    //     .context("could not connect to database_url")?;

    // We register our shared state so that handlers can get it using the State extractor.
    // Note that this will change in Axum 0.6. See more at
    // https://docs.rs/axum/0.6.0-rc.4/axum/index.html#sharing-state-with-handlers
    let app = Router::new()
        // Here we setup the routes. Note: No macros
        .route("/todos", get(get_todos).post(add_todo))
        .route("/todos/:id", delete(delete_todo).patch(update_todo).get(get_todo))
        .route("/todos/persist", post(persist))
        .with_state(db)
        // Using tower to add tracing layer
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()));

    // In practice: Use graceful shutdown.
    // Note that Axum has great examples for a log of practical scenarios,
    // including graceful shutdown (https://github.com/tokio-rs/axum/tree/main/examples)
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("Server listening on {}", addr);
    axum::Server::bind(&addr).serve(app.into_make_service()).await.unwrap();
}

/// Get list of todo items
///
/// Note how the Query extractor is used to get query parameters. Note how the State
/// extractor is used to get the database (changes in Axum 0.6 RC).
/// Extractors are technically types that implement FromRequest. You can create
/// your own extractors or use the ones provided by Axum.
async fn get_todos(pagination: Option<Query<Pagination>>, State(db): State<Db>) -> impl IntoResponse {
    let todos = db.read().await;
    let Query(pagination) = pagination.unwrap_or_default();
    // Json is an extractor and a response.
    // let item = todos.get_todos(pagination);
    let body = Json(json!({
        "message": "Get todo list successfully",
        "data": todos.get_todos(pagination),
        "meta_data": {}

    }));
    (StatusCode::OK, body).into_response()
}

/// Get a single todo item
///
/// Note how the Path extractor is used to get query parameters.
async fn get_todo(Path(id): Path<usize>, State(db): State<Db>) -> impl IntoResponse {
    let todos = db.read().await;
    let item = todos.get_todo(id);
    let mut body = Json(json!({
        "message": "Get todo item successfully",
        "data": if let Some(todo_item) = item {
            json!(todo_item)
        } else {
            json!({})
        },
        "meta_data": {}

    }));
    // tracing::debug!(item);
    if item.is_none() {
        update_resp(&mut body, "message", Value::String("Get todo item successfully".to_string()));
        (StatusCode::NOT_FOUND, body).into_response()
    } else {
        (StatusCode::OK, body).into_response()
    }
}

/// Add a new todo item
///
/// Note that this time, Json is used as an extractor. This means that the request body
/// will be deserialized into a TodoItem.
async fn add_todo(State(db): State<Db>, Json(todo): Json<TodoItem>) -> impl IntoResponse {
    let mut todos = db.write().await;
    let todo = todos.add_todo(todo);
    let body = Json(json!({
        "message": "todo item created",
        "data": todo,
        "meta_data": {}
    }));
    (StatusCode::CREATED, body).into_response()
}

/// Delete a todo item
async fn delete_todo(Path(id): Path<usize>, State(db): State<Db>) -> impl IntoResponse {
    
    let mut body = Json(json!({
        "message": "Delete todo item successfully",
        "data": {},
        "meta_data": {}

    }));
    if db.write().await.remove_todo(id).is_some() {
        (StatusCode::NO_CONTENT).into_response()
    } else {
        update_resp(&mut body, "message", Value::String("todo item not found".to_string()));
        (StatusCode::NOT_FOUND, body).into_response()
    }
}

/// Update a todo item
async fn update_todo(
    Path(id): Path<usize>,
    State(db): State<Db>,
    Json(input): Json<UpdateTodoItem>,
) -> Result<impl IntoResponse, StatusCode> {
    let mut todos = db.write().await;
    let res = todos.update_todo(&id, input);
    match res {
        Some(todo) => Ok(Json(todo.clone())),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Application-level error object
enum AppError {
    UserRepo(TodoStoreError),
}
impl From<TodoStoreError> for AppError {
    fn from(inner: TodoStoreError) -> Self {
        AppError::UserRepo(inner)
    }
}

/// Logic for turning an error into a response.
///
/// By providing this trait, handlers can return AppError and Axum will automatically
/// convert it into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::UserRepo(TodoStoreError::FileAccessError(_)) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Error while writing to file")
            },
            AppError::UserRepo(TodoStoreError::SerializationError(_)) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Error during serialization")
            },
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}

/// Persist the todo store to disk
async fn persist(State(db): State<Db>) -> Result<(), AppError> {
    tracing::debug!("Persisting todos");
    let todos = db.read().await;
    todos.persist().await?;
    Ok(())
}

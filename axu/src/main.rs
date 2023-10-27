use axum::{
    routing::get,
    Router,
};
use hyper::{
    Body,
    Request,
    Response,
};
use std::time::Instant;
use log::info;
use tower::Service;

async fn log_request(
    request: Request<Body>,
    _: axum::extract::Extension<Instant>,
) -> Result<Request<Body>, hyper::Error> {
    info!("Received request: {} {}", request.method(), request.uri());
    Ok(request)
}

async fn timing_middleware<S>(
    request: Request<Body>,
    _: axum::extract::Extension<Instant>,
    mut handler: S,
) -> Result<Response<Body>, hyper::Error>
where
    S: tower::Service<Request<Body>, Response = Response<Body>, Error = hyper::Error> + Send + 'static,
    S::Future: Send + 'static,
{
    let start = Instant::now();
    let response = handler.call(request).await?;
    let elapsed = start.elapsed();
    info!("Request completed in {:?}", elapsed);
    Ok(response)
}

async fn hello() -> &'static str {
    "Hello, Axum!"
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(hello))
        .layer(
            tower::ServiceBuilder::new()
                .layer_fn(|inner: axum::routing::BoxRoute<Body>| {
                    let instant = Instant::now();
                    move |request| {
                        let instant = instant.clone();
                        let fut = inner.call(request);
                        async move {
                            let response = fut.await?;
                            info!("Request completed in {:?}", instant.elapsed());
                            Ok(response)
                        }
                    }
                })
                .into_inner(),
        );

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
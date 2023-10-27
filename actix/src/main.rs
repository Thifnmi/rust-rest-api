use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};

#[get("/hello")]
async fn index() -> impl Responder {
    HttpResponse::Ok().body("Hello, world!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new().service(web::scope("/").service(index))
    })
    .bind("0.0.0.0:8001")?
    .run()
    .await
}
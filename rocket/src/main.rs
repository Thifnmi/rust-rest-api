#![feature(plugin)]
#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use] extern crate rocket;

#[get("/")]
fn default_page() -> String {
    format!("Wellcome to Rust REST API")
}

#[get("/<name>")]
fn name(name: String) -> String {
    format!("Hello, {}!", name)
}

#[get("/<name>/<age>")]
fn hello(name: String, age: u8) -> String {
    format!("Hello, {} year old named {}!", age, name)
}

fn main() {
    rocket::ignite().mount("/", routes![default_page])
        .mount("/", routes![name])
        .mount("/", routes![hello])
        .launch();
}
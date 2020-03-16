#![feature(plugin)]
#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
#[macro_use]
extern crate rocket_contrib;
#[macro_use]
extern crate serde_derive;

// mod actor;
mod scene;
mod image;

use rocket::config::{Config, Environment, Limits};
use std::vec::Vec;
use rocket_contrib::json::{Json, JsonValue};

#[get("/")]
fn index() -> Json<JsonValue> {
  Json(json!({
    "version": "0.1.0"
  }))
}

fn main() {
  let limits = Limits::new()
    .limit("forms", 5000000 * 1024 * 1024)
    .limit("json", 5000000 * 1024 * 1024);

  let config = Config::build(Environment::Production)
    .limits(limits)
    .unwrap();

  let app = rocket::custom(config);

  app
    .mount("/", routes![index])
    .mount("/scene", scene::get_routes())
    .mount("/image", image::get_routes())
  //.mount("/actor", actor::get_routes())
    .launch();
}

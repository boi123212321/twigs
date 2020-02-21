#![feature(plugin)]
#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;
#[macro_use] extern crate rocket_contrib;
#[macro_use] extern crate serde_derive;

mod actor;
mod image;

use std::vec::Vec;
use rocket::config::{Config, Limits, Environment};

#[get("/")]
fn index() -> &'static str {
  "Twigs 0.1"
}

fn main() {
  let limits = Limits::new()
    .limit("forms", 500 * 1024)
    .limit("json", 5000000 * 1024 * 1024);

  let config = Config::build(Environment::Production)
    .limits(limits)
    .unwrap();

  let app = rocket::custom(config);

  app
    .mount("/", routes![index])
    // .mount("/actor", actor::get_actor_routes())
    .mount("/image", image::get_image_routes())
    .launch();
}
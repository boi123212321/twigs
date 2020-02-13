#![feature(plugin)]
#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;
#[macro_use] extern crate rocket_contrib;
#[macro_use] extern crate serde_derive;

mod actor;

use std::vec::Vec;

#[get("/")]
fn index() -> &'static str {
  "Twigs 0.1"
}

fn main() {
  rocket::ignite()
    .mount("/", routes![index])
    .mount("/actor", actor::get_actor_routes())
    .launch();
}
#![feature(plugin)]
#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;
#[macro_use] extern crate rocket_contrib;
#[macro_use] extern crate serde_derive;

mod actor;

use lazy_static::lazy_static;
use rocket::http::RawStr;
use std::collections::HashMap;
use std::vec::Vec;
use std::sync::Mutex;
use rocket_contrib::json::{Json, JsonValue};
use regex::Regex;
use std::time::{Instant};

#[derive(Clone, Serialize, Deserialize)]
struct Actor {
  id: String,
  name: String,
  born_on: Option<i32>,
  aliases: Option<Vec<String>>
}

lazy_static! {
  static ref ACTORS: Mutex<HashMap<u32, Actor>> = Mutex::new(HashMap::new());
  static ref TOKENS: Mutex<HashMap<String, Vec<u32>>> = Mutex::new(HashMap::new());
}

#[get("/")]
fn index() -> &'static str {
  "Twigs 0.1"
}

#[get("/actor?<query>")]
fn get_actor(query: &RawStr) -> Json<JsonValue> {
  let s = query.url_decode().unwrap();
  println!("Searching for {}", s);
  let now = Instant::now();

  let tokens = TOKENS.lock().unwrap();
  let mut scores: HashMap<u32, u32> = HashMap::new();

  let regex = Regex::new(r"[^a-zA-Z0-9]").unwrap();
  let result = regex.replace_all(&s, " ").to_lowercase();

  for token in result.split(" ") {
    if tokens.contains_key(token) {
      let ids = tokens.get(token).unwrap();

      for id in ids.iter() {
        *scores.entry(*id).or_insert(0) += 1;
      }
    }
  }

  /* let grams = string_to_ngrams(s);
  let num_ngrams = grams.len() as u32;

  for gram in grams {
    let token: String = gram.into_iter().collect();

    if tokens.contains_key(&token) {
      let ids = tokens.get(&token).unwrap();

      for id in ids.iter() {
        *scores.entry(*id).or_insert(0) += 1;
      }
    }
  } */

  let actors = ACTORS.lock().unwrap();

  let mut real_actors: Vec<Actor> = Vec::new();

  let mut key_score_list: Vec<(u32, u32)> = Vec::new();

  for (id, score) in scores {
    key_score_list.push(
      (id, score)
    );
  }

  key_score_list.sort_by(|a,b| a.1.cmp(&b.1));;

  // Get real actors

  for tuple in key_score_list.iter_mut().rev() {
    // if tuple.1 >= num_ngrams / 2 {
      real_actors.push(
        actors.get(&tuple.0).unwrap().clone()
      );
    // }
  }

  Json(json!({
    "query": s,
    "time": {
      "sec": now.elapsed().as_secs(),
      "milli": now.elapsed().as_millis() as u64,
      "micro": now.elapsed().as_micros() as u64,
    },
    "size": real_actors.len(),
    "result": real_actors
  }))
}

/* fn string_to_ngrams(s: String) -> Vec<Vec<char>> {
  let regex = Regex::new(r"[^a-zA-Z0-9]").unwrap();
  let result = regex.replace_all(&s, " ");
  let grams: Vec<_> = result.to_lowercase().chars().ngrams(2).collect();
  grams
} */

fn process_string(s: String, id: u32) {
  let mut tokens = TOKENS.lock().unwrap();

  if s.len() > 0 {  
    // let grams = string_to_ngrams(s);
  
    // for gram in grams {
      // let token: String = gram.into_iter().collect();

    let regex = Regex::new(r"[^a-zA-Z0-9]").unwrap();
    let result = regex.replace_all(&s, " ").to_lowercase();

    for token in result.split(" ") {
      if !tokens.contains_key(token) {
        tokens.insert(
          token.to_string(), vec![id]
        );
      }
      else {
        match tokens.get_mut(token) {
          Some(vec) => {
            vec.push(id);
          },
          None => println!("Token {} does not exist", token)
        }
      }
    }
  }
}

#[post("/actors", data = "<inputs>")]
fn create_actors(inputs: Json<Vec<Actor>>) -> Json<JsonValue>  {
  let mut actors = ACTORS.lock().unwrap();

  let input_actors = inputs.into_inner();

  let mut created_actors: Vec<Actor> = Vec::new();

  for actor in input_actors.iter() {
    let id = actors.len() as u32;
    let ret_id = id.clone();
    let ret_actor = actor.clone();
    
    actors.insert(
      id,
      actor.clone()
    );

    process_string(ret_actor.name.clone(), ret_id);
    process_string(ret_actor.aliases.clone().unwrap_or(vec![]).join(" "), ret_id);
    created_actors.push(ret_actor);
  }

  Json(json!({
    "size": actors.len(),
    "actors": created_actors,
  }))
}

#[post("/actor", data = "<input>")]
fn create_actor(input: Json<Actor>) -> Json<JsonValue>  {
  let mut actors = ACTORS.lock().unwrap();

  let id = actors.len() as u32;
  let ret_id = id.clone();
  let actor = Actor { ..input.into_inner() };
  let ret_actor = actor.clone();

  actors.insert(
    id,
    actor
  );

  process_string(ret_actor.name.clone(), ret_id);
  process_string(ret_actor.aliases.clone().unwrap_or(vec![]).join(" "), ret_id);

  Json(json!({
    "size": actors.len(),
    "actor": ret_actor,
  }))
}

fn main() {
  rocket::ignite().mount("/", routes![index, create_actor, create_actors, get_actor]).launch();
}
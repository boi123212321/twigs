use lazy_static::lazy_static;
use rocket::http::RawStr;
use std::collections::HashMap;
use std::vec::Vec;
use std::sync::Mutex;
use rocket_contrib::json::{Json, JsonValue};
use regex::Regex;
use std::time::{Instant};
use rocket::http::Status;

lazy_static! {
  static ref ID_MAP: Mutex<HashMap<String, u32>> = Mutex::new(HashMap::new());
  static ref ACTORS: Mutex<HashMap<u32, Actor>> = Mutex::new(HashMap::new());
  static ref TOKENS: Mutex<HashMap<String, Vec<u32>>> = Mutex::new(HashMap::new());
}

#[derive(Clone, Serialize, Deserialize)]
struct Label {
  id: String,
  name: String,
  aliases: Option<Vec<String>>
}

#[derive(Clone, Serialize, Deserialize)]
struct Actor {
  id: String,
  name: String,
  born_on: Option<i32>,
  aliases: Vec<String>,
  labels: Vec<Label>,
  bookmark: bool,
  favorite: bool,
  rating: u8,
}

#[delete("/")]
fn clear_actors() -> Status {
  println!("Clearing actor index...");

  let mut id_map = ID_MAP.lock().unwrap();
  let mut actors = ACTORS.lock().unwrap();
  let mut tokens = TOKENS.lock().unwrap();

  // TODO: clear memory
  actors.clear();
  tokens.clear();
  id_map.clear();
  actors.shrink_to_fit();
  tokens.shrink_to_fit();
  id_map.shrink_to_fit();

  Status::Ok
}

// TODO: support vector of strings as input (from request body)
#[delete("/<id>")]
fn delete_actor(id: &RawStr) -> Status {
  println!("Deleting {}", id.as_str());

  let mut id_map = ID_MAP.lock().unwrap();
  let actor_id = id.as_str();

  if !id_map.contains_key(actor_id) {
    return Status::NotFound;
  }
  else {    
    let internal_id = id_map[actor_id];

    let mut actors = ACTORS.lock().unwrap();
    let mut tokens = TOKENS.lock().unwrap();

    actors.remove(&internal_id);

    for vec in tokens.values_mut() {
      // TODO: return internal_id from all token arrays
      for val in vec.iter_mut() {
        println!("{:?}", val);
      }
    }

    return Status::Ok;
  }
}

#[get("/?<query>&<take>&<skip>&<sort_by>&<sort_dir>&<bookmark>&<favorite>&<rating>")]
fn get_actors(query: &RawStr, take: Option<&RawStr>, skip: Option<&RawStr>, sort_by: Option<&RawStr>, sort_dir: Option<&RawStr>, bookmark: Option<&RawStr>, favorite: Option<&RawStr>, rating: Option<&RawStr>) -> Json<JsonValue> {
  let s = query.url_decode().unwrap();
  println!("Searching for {}", s);
  let now = Instant::now();

  let tokens = TOKENS.lock().unwrap();
  let mut scores: HashMap<u32, u32> = HashMap::new();

  let regex = Regex::new(r"[^a-zA-Z0-9]").unwrap();
  let result = regex.replace_all(&s, " ").to_lowercase();

  let actors = ACTORS.lock().unwrap();
  let mut real_actors: Vec<Actor> = Vec::new();

  if result.len() > 0 {
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
  
    let mut key_score_list: Vec<(u32, u32)> = Vec::new();
  
    for (id, score) in scores {
      key_score_list.push(
        (id, score)
      );
    }
  
    if sort_by.is_none() {
      // Sort by relevance
      key_score_list.sort_by(|a,b| a.1.cmp(&b.1));
    }
  
    // Get real actors
  
    let mut _skip = 0;
    
    match skip {
      Some(val) => { _skip = val.as_str().parse().expect("Not a number"); },
      None => { _skip = 0; }
    };
  
    let mut _take = 99999999999;
  
    match take {
      Some(val) => { _take = val.as_str().parse().expect("Not a number"); },
      None => { _take = 99999999999; }
    };
  
    for tuple in key_score_list.iter_mut().rev().skip(_skip).take(_take) {
      // if tuple.1 >= num_ngrams / 2 {
        real_actors.push(
          actors.get(&tuple.0).unwrap().clone()
        );
      // }
    }
  }
  else {
    for actor in actors.values() {
      real_actors.push(actor.clone());
    }
  }

  if !favorite.is_none() && favorite.unwrap() == "true" {
    real_actors.retain(|a| a.favorite);
  }

  if !bookmark.is_none() && bookmark.unwrap() == "true" {
    real_actors.retain(|a| a.bookmark);
  }

  if !rating.is_none() {
    let rating_value = rating.unwrap().parse::<u8>().expect("Invalid rating");
    real_actors.retain(|a| a.rating >= rating_value);
  }

  if !sort_by.is_none() {
    // Sort by attribute
    if sort_by.unwrap() == "age" {
      real_actors.sort_by(|a,b| {
        let a = a.born_on.unwrap_or(0);
        let b = b.born_on.unwrap_or(0);
        return a.partial_cmp(&b).unwrap();
      });
      if !sort_dir.is_none() && sort_dir.unwrap() == "desc" {
        real_actors.reverse();
      }
    }
    else if sort_by.unwrap() == "name" {
      real_actors.sort_by(|a,b| {
        let a = &a.name;
        let b = &b.name;
        return a.to_lowercase().cmp(&b.to_lowercase());
      });
      if !sort_dir.is_none() && sort_dir.unwrap() == "desc" {
        real_actors.reverse();
      }
    }
    else {
      println!("Unsupported sort attribute");
    }
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
  if s.len() > 0 {
    let mut tokens = TOKENS.lock().unwrap();
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

fn process_labels(labels: Vec<Label>, ret_id: u32) {
  for label in labels.iter() {
    process_string(label.name.clone(), ret_id);
    process_string(label.aliases.clone().unwrap_or(vec![]).join(" "), ret_id);
  }
}

#[post("/", data = "<inputs>")]
fn create_actors(inputs: Json<Vec<Actor>>) -> Json<JsonValue>  {
  let mut id_map = ID_MAP.lock().unwrap();
  
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

    id_map.insert(
      actor.id.clone(),
      id
    );
    
    process_string(ret_actor.name.clone(), ret_id);
    process_string(ret_actor.aliases.clone().join(" "), ret_id);
    process_labels(ret_actor.clone().labels.clone(), ret_id);
    created_actors.push(ret_actor);
  }

  Json(json!({
    "size": actors.len(),
    "actors": created_actors,
  }))
}

pub fn get_actor_routes() -> Vec<rocket::Route> {
  routes![get_actors, create_actors, delete_actor, clear_actors]
}
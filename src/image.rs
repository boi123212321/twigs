extern crate rust_stemmers;

use rust_stemmers::{Algorithm, Stemmer};
use lazy_static::lazy_static;
use regex::Regex;
use rocket::http::RawStr;
use rocket::http::Status;
use rocket_contrib::json::{Json, JsonValue};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;
use std::vec::Vec;

lazy_static! {
  static ref ID_MAP: Mutex<HashMap<String, u32>> = Mutex::new(HashMap::new());
  static ref IMAGES: Mutex<HashMap<u32, StoredImage>> = Mutex::new(HashMap::new());
  static ref TOKENS: Mutex<HashMap<String, Vec<u32>>> = Mutex::new(HashMap::new());
}

#[derive(Clone, Serialize, Deserialize)]
struct Aliasable {
  id: String,
  name: String,
  aliases: Option<Vec<String>>,
}

#[derive(Clone, Serialize, Deserialize)]
struct InputImage {
  id: String,
  name: String,
  added_on: i64,
  actors: Vec<Aliasable>,
  labels: Vec<Aliasable>,
  bookmark: Option<i64>,
  favorite: bool,
  rating: Option<u8>,
  scene: Option<String>,
  scene_name: Option<String>,
  studio_name: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
struct StoredImage {
  id: String,
  name: String,
  added_on: i64,
  bookmark: Option<i64>,
  favorite: bool,
  rating: Option<u8>,
  scene: Option<String>,
  actors: Vec<String>,
  labels: Vec<String>
}

fn create_storage_image(input: &InputImage) -> StoredImage {
  let actors: Vec<String> = input.actors.clone().into_iter().map(|x| x.id.clone()).collect();
  let labels: Vec<String> = input.labels.clone().into_iter().map(|x| x.id.clone()).collect();
  StoredImage {
    id: input.id.clone(),
    name: input.name.clone(),
    added_on: input.added_on,
    bookmark: input.bookmark,
    favorite: input.favorite,
    rating: input.rating,
    scene: input.scene.clone(),
    actors: actors,
    labels: labels
  }
}

#[delete("/")]
fn clear_images() -> Status {
  println!("Clearing image index...");

  let mut id_map = ID_MAP.lock().unwrap();
  let mut images = IMAGES.lock().unwrap();
  let mut tokens = TOKENS.lock().unwrap();

  images.clear();
  tokens.clear();
  id_map.clear();
  images.shrink_to_fit();
  tokens.shrink_to_fit();
  id_map.shrink_to_fit();

  Status::Ok
}

#[put("/<id>", data = "<inputs>")]
fn update_image(id: &RawStr, inputs: Json<InputImage>) -> Status {
  let id_map = ID_MAP.lock().unwrap();
  let mut images = IMAGES.lock().unwrap();
  let input_image = inputs.into_inner();

  let image_id = id.as_str();

  if id_map.contains_key(image_id) {
    let uid = id_map[image_id];
    images.insert(uid, create_storage_image(&input_image));
    process_string(input_image.name.clone(), uid);
    if !input_image.scene_name.is_none() {
      process_string(input_image.clone().scene_name.unwrap().clone(), uid);
    }
    if !input_image.studio_name.is_none() {
      process_string(input_image.clone().studio_name.unwrap().clone(), uid);
    }
    process_labels(input_image.clone().actors.clone(), uid);
    process_labels(input_image.clone().labels.clone(), uid);

    return Status::Ok;
  } else {
    return Status::NotFound;
  }
}

// TODO: support list of strings as input (from request body)
#[delete("/<id>")]
fn delete_image(id: &RawStr) -> Status {
  println!("Deleting {}", id.as_str());

  let id_map = ID_MAP.lock().unwrap();
  let image_id = id.as_str();

  if !id_map.contains_key(image_id) {
    return Status::NotFound;
  } else {
    let internal_id = id_map[image_id];

    let mut images = IMAGES.lock().unwrap();
    let mut tokens = TOKENS.lock().unwrap();

    images.remove(&internal_id);

    for vec in tokens.values_mut() {
      vec.retain(|x| *x != internal_id);
    }

    return Status::Ok;
  }
}

#[get("/info")]
fn get_images_info() -> Json<JsonValue> {
  let images = IMAGES.lock().unwrap();
  let tokens = TOKENS.lock().unwrap();

  let mut num_ref = 0;
  for vec in tokens.values() {
    num_ref += vec.len();
  }

  Json(json!({
    "size": images.len(),
    "num_tokens": tokens.len(),
    "num_references": num_ref,
    "num_references_per_token": if tokens.len() == 0 { 0 } else { num_ref / tokens.len() }
  }))
}

#[get("/?<query>&<take>&<skip>&<sort_by>&<sort_dir>&<bookmark>&<favorite>&<rating>&<include>&<exclude>&<scene>&<actors>")]
fn get_images(
    query: &RawStr,
    take: Option<&RawStr>,
    skip: Option<&RawStr>,
    sort_by: Option<&RawStr>,
    sort_dir: Option<&RawStr>,
    bookmark: Option<&RawStr>,
    favorite: Option<&RawStr>,
    rating: Option<&RawStr>,
    include: Option<&RawStr>,
    exclude: Option<&RawStr>,
    scene: Option<&RawStr>,
    actors: Option<&RawStr>,
) -> Json<JsonValue> {
    let s = query.url_decode().unwrap();
    println!("Searching images for {}", s);
    let now = Instant::now();

    let tokens = TOKENS.lock().unwrap();
    let mut scores: HashMap<u32, u32> = HashMap::new();

    let regex = Regex::new(r"[^a-zA-Z0-9]").unwrap();
    let result = regex.replace_all(&s, " ").to_lowercase();

    let images = IMAGES.lock().unwrap();
    let mut real_images: Vec<StoredImage> = Vec::new();

    let en_stemmer = Stemmer::create(Algorithm::English);

    if result.len() > 0 {
        for token in result.split(" ").map(|x| String::from(en_stemmer.stem(x))) {
            if tokens.contains_key(&token) {
                let ids = tokens.get(&token).unwrap();

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
            key_score_list.push((id, score));
        }

        if sort_by.is_none() {
            // Sort by relevance
            key_score_list.sort_by(|a, b| a.1.cmp(&b.1));
        }

        // Get real images

        for tuple in key_score_list.iter_mut().rev() {
            // if tuple.1 >= num_ngrams / 2 {
            real_images.push(images.get(&tuple.0).unwrap().clone());
            // }
        }
    } else {
        for actor in images.values() {
            real_images.push(actor.clone());
        }
    }

    if !favorite.is_none() && favorite.unwrap() == "true" {
        real_images.retain(|a| a.favorite);
    }

    if !bookmark.is_none() && bookmark.unwrap() == "true" {
        real_images.retain(|a| !a.bookmark.is_none());
    }

    if !rating.is_none() {
        let rating_value = rating.unwrap().parse::<u8>().expect("Invalid rating");
        real_images.retain(|a| a.rating.unwrap_or(0) >= rating_value);
    }

    if !include.is_none() && include.unwrap().len() > 0 {
        let include_labels = include.unwrap().as_str().split(",").collect::<Vec<&str>>();
        real_images.retain(|a| {
            for include in include_labels.iter() {
                let include_label = String::from(*include);
                let mut is_labelled = false;
                for label in a.labels.iter() {
                    if *label == include_label {
                        is_labelled = true;
                    }
                }
                if !is_labelled {
                    return false;
                }
            }
            return true;
        });
    }

    if !actors.is_none() && actors.unwrap().len() > 0 {
        let include_actors = actors.unwrap().as_str().split(",").collect::<Vec<&str>>();
        real_images.retain(|a| {
            for include in include_actors.iter() {
                let include_actor = String::from(*include);
                let mut features_actor = false;
                for actor in a.actors.iter() {
                    if *actor == include_actor {
                        features_actor = true;
                    }
                }
                if !features_actor {
                    return false;
                }
            }
            return true;
        });
    }

    if !scene.is_none() && scene.unwrap().as_str().len() > 0 {
        let scene_id = scene.unwrap().as_str();
        real_images.retain(|a| a.scene.as_ref().unwrap_or(&"".to_string()) == scene_id);
    }

    if !exclude.is_none() && exclude.unwrap().len() > 0 {
        let exclude_labels = exclude.unwrap().as_str().split(",").collect::<Vec<&str>>();
        real_images.retain(|a| {
            for exclude in exclude_labels.iter() {
                let exclude_label = String::from(*exclude);
                let mut is_labelled = false;
                for label in a.labels.iter() {
                    if *label == exclude_label {
                        is_labelled = true;
                    }
                }
                if is_labelled {
                    return false;
                }
            }
            return true;
        });
    }

    let mut _skip = 0;

    match skip {
        Some(val) => {
            _skip = val.as_str().parse().expect("Not a number");
        }
        None => {
            _skip = 0;
        }
    };

    let mut _take = 99999999999;

    match take {
        Some(val) => {
            _take = val.as_str().parse().expect("Not a number");
        }
        None => {
            _take = 99999999999;
        }
    };

    if !sort_by.is_none() {
        // Sort by attribute
        if sort_by.unwrap() == "rating" {
            real_images.sort_by(|a, b| {
                let a = a.rating;
                let b = b.rating;
                return a.partial_cmp(&b).unwrap();
            });
            if !sort_dir.is_none() && sort_dir.unwrap() == "asc" {
                real_images.reverse();
            }
        } else if sort_by.unwrap() == "addedOn" || sort_by.unwrap() == "added_on" {
            real_images.sort_by(|a, b| {
                let a = a.added_on;
                let b = b.added_on;
                return a.partial_cmp(&b).unwrap();
            });
            if !sort_dir.is_none() && sort_dir.unwrap() == "asc" {
                real_images.reverse();
            }
        } else if sort_by.unwrap() == "bookmark" {
          real_images.sort_by(|a, b| {
              let a = a.bookmark.unwrap_or(0);
              let b = b.bookmark.unwrap_or(0);
              return a.partial_cmp(&b).unwrap();
          });
          if !sort_dir.is_none() && sort_dir.unwrap() == "asc" {
              real_images.reverse();
          }
      } else if sort_by.unwrap() == "name" || sort_by.unwrap() == "alpha" {
            real_images.sort_by(|a, b| {
                let a = &a.name;
                let b = &b.name;
                return a.to_lowercase().cmp(&b.to_lowercase());
            });
            if !sort_dir.is_none() && sort_dir.unwrap() == "asc" {
                real_images.reverse();
            }
        } else {
            println!("Unsupported sort attribute");
        }
    }

    let num_hits = real_images.len();
    let page: Vec<_> = real_images
        .iter_mut()
        .rev()
        .skip(_skip)
        .take(_take)
        .collect();

    let ids: Vec<String> = page.into_iter().map(|x| x.id.clone()).collect();

    Json(json!({
      "query": s,
      "time": {
        "sec": now.elapsed().as_secs(),
        "milli": now.elapsed().as_millis() as u64,
        "micro": now.elapsed().as_micros() as u64,
      },
      "num_hits": num_hits,
      "items": ids
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
        let en_stemmer = Stemmer::create(Algorithm::English);
        let mut tokens = TOKENS.lock().unwrap();
        // let grams = string_to_ngrams(s);

        // for gram in grams {
        // let token: String = gram.into_iter().collect();

        let regex = Regex::new(r"[^a-zA-Z0-9]").unwrap();
        let result = regex.replace_all(&s, " ").to_lowercase();

        for token in result.split(" ").filter(|x| x.len() > 2).map(|x| String::from(en_stemmer.stem(x))) {
          if !tokens.contains_key(&token) {
            tokens.insert(token.to_string(), vec![id]);
          } else {
            match tokens.get_mut(&token) {
              Some(vec) => {
                vec.push(id);
              }
              None => println!("Token {} does not exist", token),
            }
          }
        }
    }
}

fn process_labels(labels: Vec<Aliasable>, ret_id: u32) {
    for label in labels.iter() {
        process_string(label.name.clone(), ret_id);
        process_string(label.aliases.clone().unwrap_or(vec![]).join(" "), ret_id);
    }
}

#[post("/", format = "json", data = "<inputs>")]
fn create_images(inputs: Json<Vec<InputImage>>) -> Json<JsonValue> {
  println!("Received new images");
  let mut id_map = ID_MAP.lock().unwrap();

  let mut images = IMAGES.lock().unwrap();
  let input_images = inputs.into_inner();

  for image in input_images.iter() {
    let id = images.len() as u32;

    images.insert(id, create_storage_image(&image));

    id_map.insert(image.id.clone(), id);

    process_string(image.name.clone(), id);
    if !image.scene_name.is_none() {
      process_string(image.clone().scene_name.unwrap().clone(), id);
    }
    if !image.studio_name.is_none() {
      process_string(image.clone().studio_name.unwrap().clone(), id);
    }
    process_labels(image.clone().actors.clone(), id);
    process_labels(image.clone().labels.clone(), id);
  }

  let tokens = TOKENS.lock().unwrap();

  let mut num_ref = 0;
  for vec in tokens.values() {
    num_ref += vec.len();
  }

  Json(json!({
    "size": images.len(),
    "num_tokens": tokens.len(),
    "num_references": num_ref,
    "num_references_per_token": if tokens.len() == 0 { 0 } else { num_ref / tokens.len() }
  }))
}

pub fn get_routes() -> Vec<rocket::Route> {
  routes![get_images, create_images, delete_image, clear_images, update_image, get_images_info]
}

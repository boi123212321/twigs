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
  static ref SCENES: Mutex<HashMap<u32, StoredScene>> = Mutex::new(HashMap::new());
  static ref TOKENS: Mutex<HashMap<String, Vec<u32>>> = Mutex::new(HashMap::new());
}

#[derive(Clone, Serialize, Deserialize)]
struct Aliasable {
  id: String,
  name: String,
  aliases: Option<Vec<String>>,
}

#[derive(Clone, Serialize, Deserialize)]
struct InputScene {
  id: String,
  name: String,
  added_on: i64,
  release_date: Option<i64>,
  bookmark: bool,
  favorite: bool,
  rating: Option<u8>,
  actors: Vec<Aliasable>,
  labels: Vec<Aliasable>,
  num_watches: u16,
  duration: Option<u16>,
  size: Option<u64>,
  studio: Option<String>,
  studio_name: Option<String>,
  resolution: Option<u16>
}

#[derive(Clone, Serialize, Deserialize)]
struct StoredScene {
  id: String,
  name: String,
  added_on: i64,
  bookmark: bool,
  favorite: bool,
  rating: Option<u8>,
  studio: Option<String>,
  actors: Vec<String>,
  labels: Vec<String>,
  num_watches: u16,
  duration: Option<u16>,
  size: Option<u64>,
  resolution: Option<u16>,
  release_date: Option<i64>
}

fn create_storage_scene(input: &InputScene) -> StoredScene {
  let actors: Vec<String> = input.actors.clone().into_iter().map(|x| x.id.clone()).collect();
  let labels: Vec<String> = input.labels.clone().into_iter().map(|x| x.id.clone()).collect();
  StoredScene {
    id: input.id.clone(),
    name: input.name.clone(),
    added_on: input.added_on,
    bookmark: input.bookmark,
    favorite: input.favorite,
    rating: input.rating,
    studio: input.studio.clone(),
    actors: actors,
    labels: labels,
    num_watches: input.num_watches,
    duration: input.duration,
    size: input.size,
    resolution: input.resolution,
    release_date: input.release_date
  }
}

#[put("/<id>", data = "<inputs>")]
fn update_scene(id: &RawStr, inputs: Json<InputScene>) -> Status {
    let id_map = ID_MAP.lock().unwrap();
    let mut scenes = SCENES.lock().unwrap();
    let input_scene = inputs.into_inner();

    let scene_id = id.as_str();

    if id_map.contains_key(scene_id) {
        let uid = id_map[scene_id];
        *scenes.get_mut(&uid).unwrap() = create_storage_scene(&input_scene);
        process_string(input_scene.name.clone(), uid);
        process_labels(input_scene.clone().labels.clone(), uid);
	process_labels(input_scene.clone().actors.clone(), uid);
        if !input_scene.studio_name.is_none() {
          process_string(input_scene.clone().studio_name.unwrap().clone(), uid);
        }
        return Status::Ok;
    } else {
        return Status::NotFound;
    }
}

// TODO: support list of strings as input (from request body)
#[delete("/<id>")]
fn delete_scene(id: &RawStr) -> Status {
  println!("Deleting {}", id.as_str());

  let id_map = ID_MAP.lock().unwrap();
  let scene_id = id.as_str();

  if !id_map.contains_key(scene_id) {
    return Status::NotFound;
  } else {
    let internal_id = id_map[scene_id];

    let mut scenes = SCENES.lock().unwrap();
    let mut tokens = TOKENS.lock().unwrap();

    scenes.remove(&internal_id);

    for vec in tokens.values_mut() {
      vec.retain(|x| *x != internal_id);
    }

    return Status::Ok;
  }
}

#[delete("/")]
fn clear_scenes() -> Status {
  println!("Clearing scene index...");

  let mut id_map = ID_MAP.lock().unwrap();
  let mut scenes = SCENES.lock().unwrap();
  let mut tokens = TOKENS.lock().unwrap();

  scenes.clear();
  tokens.clear();
  id_map.clear();
  scenes.shrink_to_fit();
  tokens.shrink_to_fit();
  id_map.shrink_to_fit();

  Status::Ok
}

#[get("/?<query>&<take>&<skip>&<sort_by>&<sort_dir>&<bookmark>&<favorite>&<rating>&<include>&<exclude>&<studio>&<actors>&<duration_min>&<duration_max>")]
fn get_scenes(
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
    studio: Option<&RawStr>,
    actors: Option<&RawStr>,
    duration_min: Option<&RawStr>,
    duration_max: Option<&RawStr>,
) -> Json<JsonValue> {
    let s = query.url_decode().unwrap();
    println!("Searching scenes for {}", s);
    let now = Instant::now();

    let tokens = TOKENS.lock().unwrap();
    let mut scores: HashMap<u32, u32> = HashMap::new();

    let regex = Regex::new(r"[^a-zA-Z0-9]").unwrap();
    let result = regex.replace_all(&s, " ").to_lowercase();

    let scenes = SCENES.lock().unwrap();
    let mut real_scenes: Vec<StoredScene> = Vec::new();

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
            key_score_list.push((id, score));
        }

        if sort_by.is_none() {
            // Sort by relevance
            key_score_list.sort_by(|a, b| a.1.cmp(&b.1));
        }

        // Get real scenes

        for tuple in key_score_list.iter_mut().rev() {
            // if tuple.1 >= num_ngrams / 2 {
            real_scenes.push(scenes.get(&tuple.0).unwrap().clone());
            // }
        }
    } else {
        for actor in scenes.values() {
            real_scenes.push(actor.clone());
        }
    }

    if !favorite.is_none() && favorite.unwrap() == "true" {
      real_scenes.retain(|a| a.favorite);
    }

    if !bookmark.is_none() && bookmark.unwrap() == "true" {
      real_scenes.retain(|a| a.bookmark);
    }

    if !rating.is_none() {
      let rating_value = rating.unwrap().parse::<u8>().expect("Invalid rating");
      real_scenes.retain(|a| a.rating.unwrap_or(0) >= rating_value);
    }

    if !duration_min.is_none() {
      let duration = duration_min.unwrap().parse::<u16>().expect("Invalid rating");
      real_scenes.retain(|a| a.duration.unwrap_or(0) >= duration);
    }

    if !duration_max.is_none() {
      let duration = duration_max.unwrap().parse::<u16>().expect("Invalid rating");
      real_scenes.retain(|a| a.duration.unwrap_or(0) <= duration);
    }

    if !studio.is_none() && studio.unwrap().as_str().len() > 0 {
      let studio_id = studio.unwrap().as_str();
      real_scenes.retain(|a| a.studio.as_ref().unwrap_or(&"".to_string()) == studio_id);
    }

    if !include.is_none() && include.unwrap().len() > 0 {
        let include_labels = include.unwrap().as_str().split(",").collect::<Vec<&str>>();
        real_scenes.retain(|a| {
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
        real_scenes.retain(|a| {
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

    if !exclude.is_none() && exclude.unwrap().len() > 0 {
        let exclude_labels = exclude.unwrap().as_str().split(",").collect::<Vec<&str>>();
        real_scenes.retain(|a| {
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
            real_scenes.sort_by(|a, b| {
                let a = a.rating;
                let b = b.rating;
                return a.partial_cmp(&b).unwrap();
            });
            if !sort_dir.is_none() && sort_dir.unwrap() == "asc" {
                real_scenes.reverse();
            }
        } else if sort_by.unwrap() == "addedOn" || sort_by.unwrap() == "added_on" {
            real_scenes.sort_by(|a, b| {
                let a = a.added_on;
                let b = b.added_on;
                return a.partial_cmp(&b).unwrap();
            });
            if !sort_dir.is_none() && sort_dir.unwrap() == "asc" {
                real_scenes.reverse();
            }
        } else if sort_by.unwrap() == "duration" {
          real_scenes.sort_by(|a, b| {
              let a = a.duration.unwrap_or(0);
              let b = b.duration.unwrap_or(0);
              return a.partial_cmp(&b).unwrap();
          });
          if !sort_dir.is_none() && sort_dir.unwrap() == "asc" {
              real_scenes.reverse();
          }
        } else if sort_by.unwrap() == "resolution" {
          real_scenes.sort_by(|a, b| {
              let a = a.resolution.unwrap_or(0);
              let b = b.resolution.unwrap_or(0);
              return a.partial_cmp(&b).unwrap();
          });
          if !sort_dir.is_none() && sort_dir.unwrap() == "asc" {
              real_scenes.reverse();
          }
        } else if sort_by.unwrap() == "size" {
          real_scenes.sort_by(|a, b| {
              let a = a.size.unwrap_or(0);
              let b = b.size.unwrap_or(0);
              return a.partial_cmp(&b).unwrap();
          });
          if !sort_dir.is_none() && sort_dir.unwrap() == "asc" {
              real_scenes.reverse();
          }
        } else if sort_by.unwrap() == "date" {
          real_scenes.sort_by(|a, b| {
              let a = a.resolution.unwrap_or(0);
              let b = b.resolution.unwrap_or(0);
              return a.partial_cmp(&b).unwrap();
          });
          if !sort_dir.is_none() && sort_dir.unwrap() == "asc" {
              real_scenes.reverse();
          }
        } else if sort_by.unwrap() == "views" {
          real_scenes.sort_by(|a, b| {
              let a = a.num_watches;
              let b = b.num_watches;
              return a.partial_cmp(&b).unwrap();
          });
          if !sort_dir.is_none() && sort_dir.unwrap() == "asc" {
              real_scenes.reverse();
          }
        } else if sort_by.unwrap() == "name" || sort_by.unwrap() == "alpha" {
            real_scenes.sort_by(|a, b| {
                let a = &a.name;
                let b = &b.name;
                return a.to_lowercase().cmp(&b.to_lowercase());
            });
            if !sort_dir.is_none() && sort_dir.unwrap() == "asc" {
                real_scenes.reverse();
            }
        } else {
            println!("Unsupported sort attribute");
        }
    }

    let num_hits = real_scenes.len();
    let page: Vec<_> = real_scenes
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

fn process_string(s: String, id: u32) {
  if s.len() > 0 {
      let mut tokens = TOKENS.lock().unwrap();
      // let grams = string_to_ngrams(s);

      // for gram in grams {
      // let token: String = gram.into_iter().collect();

      let regex = Regex::new(r"[^a-zA-Z0-9]").unwrap();
      let result = regex.replace_all(&s, " ").to_lowercase();

      for token in result.split(" ").filter(|x| x.len() > 2) {
          if !tokens.contains_key(token) {
              tokens.insert(token.to_string(), vec![id]);
          } else {
              match tokens.get_mut(token) {
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
fn create_scenes(inputs: Json<Vec<InputScene>>) -> Json<JsonValue> {
  println!("Received new scenes");
  let mut id_map = ID_MAP.lock().unwrap();

  let mut scenes = SCENES.lock().unwrap();
  let input_scenes = inputs.into_inner();

  for scene in input_scenes.iter() {
    let id = scenes.len() as u32;

    scenes.insert(id, create_storage_scene(&scene));

    id_map.insert(scene.id.clone(), id);

    process_string(scene.name.clone(), id);
    if !scene.studio_name.is_none() {
      process_string(scene.clone().studio_name.unwrap().clone(), id);
    }
    process_labels(scene.clone().actors.clone(), id);
    process_labels(scene.clone().labels.clone(), id);
  }

  let tokens = TOKENS.lock().unwrap();

  let mut num_ref = 0;
  for vec in tokens.values() {
    num_ref += vec.len();
  }

  Json(json!({
    "size": scenes.len(),
    "num_tokens": tokens.len(),
    "num_references": num_ref,
    "num_references_per_token": if tokens.len() == 0 { 0 } else { num_ref / tokens.len() }
  }))
}

pub fn get_routes() -> Vec<rocket::Route> {
  routes![get_scenes, create_scenes, delete_scene, clear_scenes]
}

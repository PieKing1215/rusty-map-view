use std::collections::{HashMap, HashSet};

use self::room::Room;

pub mod item;
pub mod room;
pub mod transition;

pub struct MapData {
    pub areas: HashMap<String, String>,
    pub rooms: HashMap<String, Room>,
}

pub struct RandoData {
    pub transition_map: HashMap<String, String>,
    pub visited_transitions: HashSet<String>,
    pub room_positions: HashMap<String, (f32, f32)>,
}

pub fn load_mapdata(json_source: &str) -> Result<MapData, String> {
    let json = json::parse(json_source).map_err(|je| je.to_string())?;

    let areas: HashMap<String, String> = json["areas"]
        .entries()
        .map(|(k, v)| (k.into(), v.as_str().unwrap().into()))
        .collect();

    let rooms: HashMap<String, Room> = json["rooms"]
        .entries()
        .map(|(k, v)| {
            Ok((
                k.into(),
                Room::try_from(v).map_err(|s| format!("{k}: {s}"))?,
            ))
        })
        .collect::<Result<_, String>>()?;

    Ok(MapData { areas, rooms })
}

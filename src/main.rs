#![allow(clippy::expect_fun_call)]

pub mod data;
pub mod settings;
pub mod state;
pub mod util;

use std::{
    collections::{BTreeSet, HashMap, HashSet},
    sync::mpsc::{self, Receiver, SyncSender},
    thread::JoinHandle,
    time::Instant,
};

use data::MapData;
use ggez::{
    conf::{WindowMode, WindowSetup},
    event::{self, MouseButton},
    graphics::{self, Color, DrawParam, Drawable, Rect},
    input::mouse::CursorIcon,
    mint::Point2,
    Context, GameError, GameResult,
};
use ggez_egui::EguiBackend;
use json::JsonValue;
use parity_ws::{Message, Sender};
use settings::Settings;
use util::{color_ext::ColorExt, split::GetSplit, transform_stack::TransformStack};

use crate::{
    data::{transition::Transition, RandoData},
    state::{Camera, CameraTarget, GameState, LoadedState},
    util::rect_ext::RectExt,
};

struct MainState {
    pos_x: f32,
    circle: graphics::Mesh,
    map_data: MapData,
    recv: Receiver<JsonValue>,
    shutdown: SyncSender<()>,
    listen_thread: Option<JoinHandle<()>>,
    shutdown_thread: Option<JoinHandle<()>>,
    game_state: GameState,
    last_transition_time: Instant,
    asset_cache: HashMap<String, graphics::Image>,
    click_start_x: f32,
    click_start_y: f32,
    path_target: Option<String>,
    highlight_path: Option<Vec<String>>,
    egui_backend: EguiBackend,
    egui_ctx: Option<egui::Context>,
    settings: Settings,
}

impl MainState {
    fn new(ctx: &mut Context) -> GameResult<MainState> {
        let circle = graphics::Mesh::new_circle(
            ctx,
            graphics::DrawMode::fill(),
            [0.0, 0.0],
            20.0,
            2.0,
            Color::from_rgba_u32(0xffffff40),
        )?;

        println!("Loading map data...");
        let map_data = data::load_mapdata(include_str!("../res/mapdata.json"))
            .map_err(GameError::CustomError)?;

        let (send, recv) = mpsc::sync_channel(10);
        let (send_shutdown, recv_shutdown) = mpsc::sync_channel(1);
        let (send_out, recv_out) = mpsc::sync_channel(1);

        let shutdown_thread = std::thread::spawn(move || {
            let out: Sender = recv_out.recv().unwrap();
            recv_shutdown.recv().unwrap();
            out.close(parity_ws::CloseCode::Normal).unwrap();
        });

        let listen_thread = std::thread::spawn(move || {
            if let Err(error) = parity_ws::connect("ws://localhost:7900/ws", |out| {
                send_out.send(out).unwrap();

                |msg: Message| {
                    if let Ok(data) = msg.into_text() {
                        println!("recv {} bytes", data.len());
                        match json::parse(&data) {
                            Ok(json) => {
                                send.send(json).unwrap();
                            },
                            Err(err) => eprintln!("json::parse: {err}"),
                        }
                    }

                    Ok(())
                }
            }) {
                println!("Failed to create WebSocket: {:?}", error);
            }
        });

        let egui_backend = EguiBackend::default();
        Ok(MainState {
            pos_x: 0.0,
            circle,
            recv,
            shutdown: send_shutdown,
            listen_thread: Some(listen_thread),
            shutdown_thread: Some(shutdown_thread),
            game_state: GameState::Unloaded,
            map_data,
            last_transition_time: Instant::now(),
            asset_cache: HashMap::new(),
            click_start_x: 0.0,
            click_start_y: 0.0,
            path_target: None,
            highlight_path: None,
            settings: Settings::default(),
            egui_ctx: None,
            egui_backend,
        })
    }

    fn on_message(&mut self, json: JsonValue, ctx: &mut Context) -> GameResult {
        // println!("{}", json["type"]);
        match json["type"].as_str() {
            Some("loadSave") => {
                self.load_save(&json["data"]).expect("Failed to load data");
            },
            Some("unloadSave") => {
                self.game_state = GameState::Unloaded;
            },
            Some("playerMove") => {
                if let GameState::Loaded(state) = &mut self.game_state {
                    state.current_room = json["newRoom"]
                        .as_str()
                        .expect(
                            format!("Missing/Invalid field 'newRoom': {}", json["newRoom"])
                                .as_str(),
                        )
                        .into();
                    println!("Changed room: {}", state.current_room);
                    state.player_x = json["x"]
                        .as_f32()
                        .expect(format!("Missing/Invalid field 'x': {}", json["x"]).as_str());
                    state.player_y = json["y"]
                        .as_f32()
                        .expect(format!("Missing/Invalid field 'y': {}", json["y"]).as_str());
                    state.rando_data.room_positions.clear();
                    self.last_transition_time = Instant::now();
                }
            },
            Some("playerPos") => {
                if let GameState::Loaded(state) = &mut self.game_state {
                    state.player_x = json["x"]
                        .as_f32()
                        .expect(format!("Missing/Invalid field 'x': {}", json["x"]).as_str());
                    state.player_y = json["y"]
                        .as_f32()
                        .expect(format!("Missing/Invalid field 'y': {}", json["y"]).as_str());
                }
            },
            Some("revealTransition") => {
                let to: String = json["to"]
                    .as_str()
                    .expect(format!("Missing/Invalid field 'to': {}", json["to"]).as_str())
                    .into();
                if let GameState::Loaded(state) = &mut self.game_state {
                    state.rando_data.visited_transitions.insert(to.clone());
                    if let Some(from) = state.rando_data.transition_map.get(&to) {
                        state.rando_data.visited_transitions.insert(from.clone());
                        println!("Reveal transition: {}", from);
                    }
                }
                println!("Reveal transition: {}", to);
            },
            Some("getItem") => {
                let item: String = json["item"]
                    .as_str()
                    .expect(format!("Missing/Invalid field 'item': {}", json["item"]).as_str())
                    .into();
                let location: String = json["location"]
                    .as_str()
                    .expect(
                        format!("Missing/Invalid field 'location': {}", json["location"]).as_str(),
                    )
                    .into();
                println!("Got item: {} @ {}", item, location);
            },
            Some("asset") => {
                let name: String = json["name"]
                    .as_str()
                    .expect(format!("Missing/Invalid field 'name': {}", json["name"]).as_str())
                    .into();
                let data: String = json["data"]
                    .as_str()
                    .expect(format!("Missing/Invalid field 'data': {}", json["data"]).as_str())
                    .into();
                match base64::decode(data) {
                    Ok(data) => {
                        let decoded = image::load_from_memory(&data).map_err(|_| {
                            GameError::ResourceLoadError("image::load_from_memory failed".into())
                        })?;
                        let rgba8 = decoded.to_rgba8();
                        let (width, height) = (rgba8.width(), rgba8.height());

                        let img = graphics::Image::from_rgba8(
                            ctx,
                            width as u16,
                            height as u16,
                            rgba8.as_ref(),
                        )?;

                        self.asset_cache.insert(name, img);
                    },
                    Err(e) => {
                        println!("{e:?}");
                    },
                }
            },
            Some(s) => println!("Unimplemented message type: {s}"),
            _ => println!("Message missing type!"),
        }

        Ok(())
    }

    fn load_save(&mut self, data: &JsonValue) -> Result<(), String> {
        let hk_ver: String = data["playerData"]["version"]
            .as_str()
            .expect(
                format!(
                    "Missing/Invalid field 'playerData.version': {}",
                    data["version"]
                )
                .as_str(),
            )
            .into();
        println!("hk_ver = {hk_ver}");

        let rando_data = data["PolymorphicModData"]["RandomizerMod"]
            .as_str()
            .ok_or_else(|| "Missing data.PolymorphicModData.RandomizerMod".into())
            .and_then(|raw_json| json::parse(raw_json).map_err(|json_err| json_err.to_string()))?;
        let rando_ctx = data["PolymorphicModData"]["context"]
            .as_str()
            .ok_or_else(|| "Missing data.PolymorphicModData.context".into())
            .and_then(|raw_json| json::parse(raw_json).map_err(|json_err| json_err.to_string()))?;

        let mut transition_map = HashMap::new();

        // fill with vanilla to start
        for (id, room) in &self.map_data.rooms {
            for (tr_id, tr) in &room.transitions {
                if let Some(to) = &tr.to {
                    transition_map.insert(format!("{id}[{tr_id}]"), to.clone());
                }
            }
        }

        // update with randomized data
        for obj in rando_ctx["transitionPlacements"].members() {
            let src = obj["Source"]["Name"].as_str().unwrap().into();
            let dst = obj["Target"]["Name"].as_str().unwrap().into();
            transition_map.insert(src, dst);
        }

        let mut visited_transitions = HashSet::new();
        for (src, dst) in rando_data["TrackerData"]["visitedTransitions"].entries() {
            visited_transitions.insert(src.into());
            visited_transitions.insert(dst.as_str().unwrap().into());
        }

        self.game_state = GameState::Loaded(LoadedState {
            current_room: rando_ctx["StartDef"]["SceneName"]
                .as_str()
                .expect(
                    format!(
                        "Missing/Invalid field 'rando_ctx.StartDef.SceneName': {}",
                        rando_ctx["StartDef"]["SceneName"]
                    )
                    .as_str(),
                )
                .into(),
            player_x: 0.0,
            player_y: 0.0,
            rando_data: RandoData {
                transition_map,
                visited_transitions,
                room_positions: HashMap::new(),
            },
            camera: Camera { x: 0.0, y: 0.0, target: CameraTarget::Player },
            hovered_room: None,
            selected_room: None,
            dragging_room: false,
        });

        Ok(())
    }

    fn update_room_positions(&mut self) {
        if let GameState::Loaded(state) = &mut self.game_state {
            let rough_factor = 1.0
                - (Instant::now()
                    .saturating_duration_since(self.last_transition_time)
                    .as_secs_f32()
                    / 1.0
                    - 0.5)
                    .powi(2)
                    .clamp(0.0, 1.0);
            // let rough_factor = 1.0;

            #[allow(clippy::needless_collect)] // actually needed
            let v: Vec<_> = state.rando_data.room_positions.keys().cloned().collect();
            for key in v.into_iter() {
                if key == state.current_room {
                    let (this_x, this_y) = state.rando_data.room_positions.get_mut(&key).unwrap();
                    *this_x = 0.0;
                    *this_y = 0.0;
                } else if state.selected_room.as_ref() == Some(&key) && state.dragging_room {
                    // don't move
                } else if let Some((cur_room, other_rooms)) =
                    self.map_data.rooms.split(&key).as_deref()
                {
                    let bounds = cur_room.calc_bounds();

                    let (this_x, this_y) = *state.rando_data.room_positions.get(&key).unwrap();

                    let mut move_x = 0.0;
                    let mut move_y = 0.0;
                    // let mut i = 0;

                    // try to line up transitions
                    for (k, tr) in &cur_room.transitions {
                        let transition = format!("{}[{k}]", key);
                        if state.rando_data.visited_transitions.contains(&transition) {
                            if let Some((to_room, to_transition_k)) =
                                Transition::get_transition_info(
                                    state
                                        .rando_data
                                        .transition_map
                                        .get(&transition)
                                        .unwrap_or(&transition),
                                )
                            {
                                if state.rando_data.room_positions.contains_key(&to_room) {
                                    if let Some(next_room) = other_rooms.get(&to_room) {
                                        let next_bounds = next_room.calc_bounds();

                                        if let Some(to_transition) =
                                            next_room.transitions.get(&to_transition_k)
                                        {
                                            let (other_x, other_y) = *state
                                                .rando_data
                                                .room_positions
                                                .get(&to_room)
                                                .unwrap();

                                            // move so src lines up with dst
                                            let strength = 0.005 + 0.4 * rough_factor;
                                            let mut strength_x;
                                            let mut strength_y;

                                            let dx = (-tr.x + to_transition.x) - this_x + other_x;
                                            let dy = (-(bounds.h - tr.y)
                                                + (next_bounds.h - to_transition.y))
                                                - this_y
                                                + other_y;

                                            if (k.starts_with("right")
                                                && to_transition_k.starts_with("left"))
                                                || (k.starts_with("left")
                                                    && to_transition_k.starts_with("right"))
                                            {
                                                strength_x =
                                                    ((dx.abs() - 200.0) / 200.0).clamp(0.0, 0.5);
                                                if (k.starts_with("right") && dx < 0.0)
                                                    || (k.starts_with("left") && dx > 0.0)
                                                {
                                                    strength_x = 2.0;
                                                }
                                                strength_y = 2.0;
                                            } else if (k.starts_with("top")
                                                && to_transition_k.starts_with("bot"))
                                                || (k.starts_with("bot")
                                                    && to_transition_k.starts_with("top"))
                                            {
                                                strength_x = 2.0;
                                                strength_y =
                                                    ((dy.abs() - 200.0) / 200.0).clamp(0.0, 0.5);
                                                if (k.starts_with("top") && dy > 0.0)
                                                    || (k.starts_with("bot") && dy < 0.0)
                                                {
                                                    strength_y = 2.0;
                                                }
                                            } else {
                                                strength_x =
                                                    ((dx.abs() - 400.0) / 400.0).clamp(0.0, 0.5);
                                                strength_y =
                                                    ((dy.abs() - 400.0) / 400.0).clamp(0.0, 0.5);
                                            }

                                            move_x += dx * strength * strength_x;
                                            move_y += dy * strength * strength_y;

                                            // i += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // remove intersections
                    for (other_key, other_room) in other_rooms.iter() {
                        if state.rando_data.room_positions.contains_key(other_key) {
                            let other_bounds = other_room.calc_bounds();
                            let (other_x, other_y) =
                                *state.rando_data.room_positions.get(other_key).unwrap();

                            let mut tr_my_bounds = bounds; // copy
                            tr_my_bounds.translate([this_x, this_y + bounds.h]);
                            tr_my_bounds.inflate(10.0);

                            let mut tr_other_bounds = other_bounds; // copy
                            tr_other_bounds.translate([other_x, other_y + other_bounds.h]);
                            tr_other_bounds.inflate(10.0);

                            if tr_my_bounds.overlaps(&tr_other_bounds) {
                                let ox1 = tr_my_bounds.left().max(tr_other_bounds.left());
                                let oy1 = tr_my_bounds.top().max(tr_other_bounds.top());
                                let ox2 = tr_my_bounds.right().min(tr_other_bounds.right());
                                let oy2 = tr_my_bounds.bottom().min(tr_other_bounds.bottom());
                                let overlap_rect = Rect::new(ox1, oy1, ox2 - ox1, oy2 - oy1);

                                move_x += (tr_my_bounds.center().x - overlap_rect.center().x)
                                    * 0.00005
                                    * overlap_rect.w
                                    * overlap_rect.h
                                    * (1.0 - rough_factor);
                                move_y += (tr_my_bounds.center().y - overlap_rect.center().y)
                                    * 0.00005
                                    * overlap_rect.w
                                    * overlap_rect.h
                                    * (1.0 - rough_factor);
                            }

                            // wide area

                            let mut tr_my_bounds = bounds; // copy
                            tr_my_bounds.translate([this_x, this_y + bounds.h]);
                            tr_my_bounds.inflate(25.0);

                            let mut tr_other_bounds = other_bounds; // copy
                            tr_other_bounds.translate([other_x, other_y + other_bounds.h]);
                            tr_other_bounds.inflate(25.0);

                            if tr_my_bounds.overlaps(&tr_other_bounds) {
                                let ox1 = tr_my_bounds.left().max(tr_other_bounds.left());
                                let oy1 = tr_my_bounds.top().max(tr_other_bounds.top());
                                let ox2 = tr_my_bounds.right().min(tr_other_bounds.right());
                                let oy2 = tr_my_bounds.bottom().min(tr_other_bounds.bottom());
                                let overlap_rect = Rect::new(ox1, oy1, ox2 - ox1, oy2 - oy1);

                                move_x += (tr_my_bounds.center().x - tr_other_bounds.center().x)
                                    * 0.0000005
                                    * overlap_rect.w
                                    * overlap_rect.h
                                    * (1.0 - rough_factor);
                                move_y += (tr_my_bounds.center().y - tr_other_bounds.center().y)
                                    * 0.0000005
                                    * overlap_rect.w
                                    * overlap_rect.h
                                    * (1.0 - rough_factor);
                            }
                        }
                    }

                    let (this_x, this_y) = state.rando_data.room_positions.get_mut(&key).unwrap();
                    *this_x = (*this_x + move_x.clamp(-100.0, 100.0)).clamp(-1000.0, 1000.0);
                    *this_y = (*this_y + move_y.clamp(-100.0, 100.0)).clamp(-1000.0, 1000.0);
                }
            }
        }
    }

    fn get_room_at_window_position(
        &self,
        ctx: &Context,
        pos: impl Into<Point2<f32>>,
    ) -> Option<&String> {
        let pos = pos.into();

        if let GameState::Loaded(state) = &self.game_state {
            let mut transform = TransformStack::new();

            transform.push();
            transform.translate(
                graphics::window(ctx).inner_size().width as f32 / 2.0,
                graphics::window(ctx).inner_size().height as f32 / 2.0,
            );
            transform.translate(-state.camera.x, -state.camera.y);

            for (room_key, (x, y)) in &state.rando_data.room_positions {
                if let Some(cur_room) = self.map_data.rooms.get(room_key) {
                    transform.push();

                    let mut bounds = cur_room.calc_bounds();
                    let orig_h = bounds.h;
                    bounds.inflate(5.0);

                    transform.translate(*x, *y);

                    let (rel_x, rel_y) = transform.inv_transform((pos.x, pos.y));

                    if bounds.contains([rel_x, rel_y - orig_h]) {
                        return Some(room_key);
                    }

                    transform.pop();
                }
            }

            transform.pop();
        }

        None
    }

    #[allow(clippy::ptr_arg)]
    fn find_path(&self, src: &String, dst: &String) -> Option<Vec<String>> {
        if let GameState::Loaded(state) = &self.game_state {
            let mut dist_from_src: HashMap<String, f32> = HashMap::new();
            let mut prev_transition: HashMap<String, String> = HashMap::new();

            dist_from_src.insert(src.clone(), 0.0);

            let mut unvisited: Vec<String> = self.map_data.rooms.keys().cloned().collect();

            while let Some((idx, _)) = unvisited
                .iter()
                .enumerate()
                .filter(|(_, r)| dist_from_src.contains_key(*r))
                .min_by(|a, b| {
                    dist_from_src
                        .get(a.1)
                        .unwrap()
                        .partial_cmp(dist_from_src.get(b.1).unwrap())
                        .unwrap()
                })
            {
                let visiting = unvisited.remove(idx);
                // println!("visiting {visiting}");

                if &visiting == dst {
                    let mut path = Vec::new();
                    let mut prev_room = visiting;

                    while let Some(prev_tr) = prev_transition.get(&prev_room) {
                        path.push(prev_tr.clone());
                        prev_room = Transition::get_transition_info(prev_tr).unwrap().0;
                    }

                    path.reverse();
                    return Some(path);
                }

                let room = self.map_data.rooms.get(&visiting).unwrap();

                let dist_to_cur = *dist_from_src.get(&visiting).unwrap();

                for tr_key in room.transitions.keys() {
                    let cost = dist_to_cur + 1.0;

                    let transition = format!("{}[{tr_key}]", visiting);
                    if state.rando_data.visited_transitions.contains(&transition) {
                        if let Some((to_room, _to_transition)) = Transition::get_transition_info(
                            state
                                .rando_data
                                .transition_map
                                .get(&transition)
                                .unwrap_or(&transition),
                        ) {
                            if unvisited.contains(&to_room) {
                                if let Some(v) = dist_from_src.get(&to_room).cloned() {
                                    if cost < v {
                                        dist_from_src.insert(to_room.clone(), cost);
                                        prev_transition.insert(to_room, transition);
                                    }
                                } else {
                                    dist_from_src.insert(to_room.clone(), cost);
                                    prev_transition.insert(to_room, transition);
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }
}

impl event::EventHandler<ggez::GameError> for MainState {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        let egui_ctx = self.egui_backend.ctx();
        self.egui_ctx = Some((&*egui_ctx).clone());
        egui::Window::new("Rusty Map View").show(&egui_ctx, |ui| {
            if ui.button("quit").clicked() {
                ggez::event::quit(ctx);
            }
        });

        egui::Window::new("All Settings").show(&egui_ctx, |ui| {
            self.settings.fill_debug_egui(ui);
        });

        self.pos_x = self.pos_x % 800.0 + 1.0;

        if let Ok(msg) = self.recv.try_recv() {
            self.on_message(msg, ctx)?;
        }

        self.update_room_positions();

        if let GameState::Loaded(state) = &mut self.game_state {
            state.update(&self.map_data);
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        graphics::clear(ctx, [0.0, 0.0, 0.0, 1.0].into());

        graphics::draw(
            ctx,
            &self.circle,
            DrawParam::default().dest([self.pos_x, 380.0]),
        )?;

        let hovered_room = self
            .get_room_at_window_position(ctx, ggez::input::mouse::position(ctx))
            .cloned();

        if let GameState::Loaded(state) = &mut self.game_state {
            state.hovered_room = hovered_room;

            let mut transform = TransformStack::new();

            let mut render_rooms = BTreeSet::new();
            render_rooms.insert(state.current_room.clone());
            let depth = self.settings.depth;
            for _ in 0..depth {
                for key in render_rooms.clone() {
                    if let Some(room) = self.map_data.rooms.get(&key) {
                        for k in room.transitions.keys() {
                            let transition = format!("{}[{k}]", key);
                            if state.rando_data.visited_transitions.contains(&transition) {
                                if let Some((to_room, _to_transition)) =
                                    Transition::get_transition_info(
                                        state
                                            .rando_data
                                            .transition_map
                                            .get(&transition)
                                            .unwrap_or(&transition),
                                    )
                                {
                                    // if to_room == "Fungus1_19" || to_room == "Mines_20" {
                                    render_rooms.insert(to_room);
                                    // }
                                }
                            }
                        }
                    }
                }
            }

            transform.push();
            transform.translate(
                graphics::window(ctx).inner_size().width as f32 / 2.0,
                graphics::window(ctx).inner_size().height as f32 / 2.0,
            );
            transform.translate(-state.camera.x, -state.camera.y);
            // transform.scale(2.0, 2.0);
            // if let Some(cur_room) = self.map_data.rooms.get(&state.current_room) {
            //     let bounds = cur_room.calc_bounds();
            //     transform.translate(-bounds.center().x, -(bounds.h + bounds.center().y));
            // }

            for key in &render_rooms {
                if let Some((cur_room, other_rooms)) = self.map_data.rooms.split(key).as_deref_mut()
                {
                    transform.push();

                    let bounds = cur_room.calc_bounds();

                    let (x, y) = *state
                        .rando_data
                        .room_positions
                        .entry(key.clone())
                        .or_insert_with(|| (0.0, 0.0));
                    transform.translate(x, y);

                    cur_room.draw(
                        ctx,
                        transform.clone(),
                        key,
                        &state.rando_data,
                        &self.asset_cache,
                        state.hovered_room.as_ref().map(|k| k == key),
                        state.selected_room.as_ref().map(|k| k == key),
                        &self.highlight_path,
                        &self.settings,
                    )?;

                    for (k, tr) in &cur_room.transitions {
                        let transition = format!("{}[{k}]", key);
                        if state.rando_data.visited_transitions.contains(&transition) {
                            if let Some((to_room, to_transition_key)) =
                                Transition::get_transition_info(
                                    state
                                        .rando_data
                                        .transition_map
                                        .get(&transition)
                                        .unwrap_or(&transition),
                                )
                            {
                                if state.rando_data.room_positions.contains_key(&to_room) {
                                    if let Some(next_room) = other_rooms.get(&to_room) {
                                        let next_bounds = next_room.calc_bounds();

                                        if let Some(to_transition) =
                                            next_room.transitions.get(&to_transition_key)
                                        {
                                            // move so src lines up with dst
                                            // dx += ((-tr.x + to_transition.x) - this_x) * 0.0025;
                                            // dy += ((-(bounds.h - tr.y) + (next_bounds.h - to_transition.y)) - this_y) * 0.0025;

                                            let (x2, y2) = state
                                                .rando_data
                                                .room_positions
                                                .get(&to_room)
                                                .unwrap();

                                            let to_transition_id =
                                                format!("{}[{to_transition_key}]", to_room);

                                            let mut color =
                                                graphics::Color::from_rgba(64, 64, 192, 127);
                                            if let Some(path) = &self.highlight_path {
                                                if let Some(i) = path.iter().position(|path_tr| {
                                                    path_tr == &transition
                                                        || path_tr == &to_transition_id
                                                }) {
                                                    let thru =
                                                        ((ggez::timer::time_since_start(ctx)
                                                            .as_secs_f32()
                                                            + i as f32)
                                                            / 0.25)
                                                            .sin()
                                                            .max(0.25);
                                                    color = color.lerp(
                                                        &Color::from_rgb(255, 100, 160),
                                                        thru,
                                                    );
                                                }
                                            }

                                            let points = [
                                                [tr.x, bounds.h - tr.y],
                                                [
                                                    -x + *x2 + to_transition.x,
                                                    -y + *y2 + (next_bounds.h - to_transition.y),
                                                ],
                                            ];
                                            // TODO: don't need this check in ggez 0.8
                                            if (points[0][0] - points[1][0]).abs() > 0.1
                                                || (points[0][1] - points[1][1]).abs() > 0.1
                                            {
                                                graphics::Mesh::new_line(ctx, &points, 2.0, color)?
                                                    .draw(ctx, (&transform).into())?;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    transform.pop()
                }
            }

            if let Some(cur_room) = self.map_data.rooms.get(&state.current_room) {
                let bounds = cur_room.calc_bounds();

                transform.push();

                let (x, y) = *state
                    .rando_data
                    .room_positions
                    .entry(state.current_room.clone())
                    .or_insert_with(|| (0.0, 0.0));
                transform.translate(x, y);

                transform.push();
                transform.translate(state.player_x, bounds.h - state.player_y);

                if let Some(img) = self.asset_cache.get("Map_Knight_Pin_Compass") {
                    transform.push();
                    transform.scale(0.33, 0.33);
                    transform.translate(-(img.width() as f32) / 2.0, -(img.height() as f32) / 2.0);

                    graphics::draw(ctx, img, &transform)?;

                    transform.pop();
                } else {
                    let player = graphics::Mesh::new_circle(
                        ctx,
                        graphics::DrawMode::fill(),
                        [0.0, 0.0],
                        3.0,
                        2.0,
                        Color::WHITE,
                    )?;
                    graphics::draw(ctx, &player, &transform)?;
                }

                transform.pop();

                transform.pop();
            }

            transform.pop();

            graphics::Text::new(format!("Hovered: {:?}", state.hovered_room))
                .draw(ctx, DrawParam::default().dest([8.0, 20.0]))?;

            graphics::Text::new(format!("Path Target: {:?}", self.path_target))
                .draw(ctx, DrawParam::default().dest([8.0, 40.0]))?;

            ggez::input::mouse::set_cursor_type(
                ctx,
                if state.hovered_room.is_some() {
                    CursorIcon::Hand
                } else {
                    CursorIcon::Default
                },
            );

            if let Some(path) = &self.highlight_path {
                graphics::Text::new(format!("Path: {:?}", path))
                    .draw(ctx, DrawParam::default().dest([8.0, 60.0]))?;
            }
        }

        //img.draw(&mut canvas, [100.0, 100.0].into());

        graphics::Text::new(format!("{:.0} FPS", ggez::timer::fps(ctx)))
            .set_bounds([60.0, 20.0], graphics::Align::Right)
            .draw(
                ctx,
                DrawParam::default().dest([
                    graphics::window(ctx).inner_size().width as f32 - 60.0 - 2.0,
                    2.0,
                ]),
            )?;

        let room = if let GameState::Loaded(state) = &mut self.game_state {
            &state.current_room
        } else {
            "Not Loaded"
        };
        graphics::Text::new(format!("Current: {room}"))
            .draw(ctx, DrawParam::default().dest([8.0, 2.0]))?;

        graphics::draw(ctx, &self.egui_backend, ([0.0, 0.0],))?;

        graphics::present(ctx)?;

        Ok(())
    }

    fn quit_event(&mut self, _ctx: &mut Context) -> bool {
        if self.shutdown_thread.is_some() {
            println!("Closing connection...");
            self.shutdown.send(()).unwrap();
            self.shutdown_thread.take().unwrap().join().unwrap();
            self.listen_thread.take().unwrap().join().unwrap();
            println!("Done");
        }
        false
    }

    fn mouse_button_down_event(
        &mut self,
        ctx: &mut Context,
        button: event::MouseButton,
        x: f32,
        y: f32,
    ) {
        self.egui_backend.input.mouse_button_down_event(button);

        if !self
            .egui_ctx
            .as_ref()
            .map_or(false, |ectx| ectx.wants_pointer_input())
        {
            self.click_start_x = x;
            self.click_start_y = y;

            if button == MouseButton::Left {
                let hovered_room = self.get_room_at_window_position(ctx, [x, y]).cloned();

                if let GameState::Loaded(state) = &mut self.game_state {
                    state.dragging_room = true;

                    state.hovered_room = hovered_room;
                    state.selected_room = state.hovered_room.clone();
                }
            } else if button == MouseButton::Right {
                let hovered_room = self.get_room_at_window_position(ctx, [x, y]).cloned();

                if let GameState::Loaded(state) = &mut self.game_state {
                    state.hovered_room = hovered_room;
                    self.path_target = state.hovered_room.clone();
                    let src = state
                        .selected_room
                        .clone()
                        .unwrap_or_else(|| state.current_room.clone());
                    let dst = self.path_target.clone();
                    self.highlight_path = dst.and_then(|dst| self.find_path(&src, &dst));

                    // state.selected_room = state.hovered_room.clone();
                    // if let Some(r) = state.hovered_room.clone() {
                    //     state.camera.target = CameraTarget::Room(r);
                    // }
                }
            }
        }
    }

    fn mouse_button_up_event(&mut self, _ctx: &mut Context, button: MouseButton, _x: f32, _y: f32) {
        self.egui_backend.input.mouse_button_up_event(button);

        // if button == MouseButton::Left && (self.click_start_x - x).abs() <= 2.0 && (self.click_start_y - y).abs() <= 2.0 {
        //     let hovered_room = self.get_room_at_window_position(ctx, [x, y]).cloned();

        //     if let GameState::Loaded(state) = &mut self.game_state {
        //         state.hovered_room = hovered_room;
        //         state.selected_room = state.hovered_room.clone();
        //     }
        // }

        #[allow(clippy::collapsible_if)]
        if !self
            .egui_ctx
            .as_ref()
            .map_or(false, |ectx| ectx.wants_pointer_input())
        {
            if button == MouseButton::Left {
                if let GameState::Loaded(state) = &mut self.game_state {
                    state.dragging_room = false;
                }
            }
        }
    }

    fn mouse_motion_event(&mut self, ctx: &mut Context, x: f32, y: f32, dx: f32, dy: f32) {
        self.egui_backend.input.mouse_motion_event(x, y);

        if !self
            .egui_ctx
            .as_ref()
            .map_or(false, |ectx| ectx.wants_pointer_input())
        {
            if let GameState::Loaded(state) = &mut self.game_state {
                if ggez::input::mouse::button_pressed(ctx, MouseButton::Left) {
                    if let Some(sel_room) = &state.selected_room {
                        if let Some((x, y)) = state.rando_data.room_positions.get_mut(sel_room) {
                            *x += dx;
                            *y += dy;
                        }
                    }
                }
            }
        }
    }

    // TODO: not needed in ggez 0.8
    fn resize_event(&mut self, ctx: &mut Context, width: f32, height: f32) {
        let rect = graphics::Rect::new(0.0, 0.0, width as f32, height as f32);
        graphics::set_screen_coordinates(ctx, rect).unwrap();
    }
}

pub fn main() -> GameResult {
    let cb = ggez::ContextBuilder::new("rusty-map-view", "PieKing1215")
        .window_setup(WindowSetup::default().title("rusty-map-view").vsync(false))
        .window_mode(WindowMode::default().resizable(true));
    let (mut ctx, event_loop) = cb.build()?;
    let state = MainState::new(&mut ctx)?;
    event::run(ctx, event_loop, state)
}

use crate::data::{MapData, RandoData};

#[allow(clippy::large_enum_variant)]
pub enum GameState {
    Unloaded,
    Loaded(LoadedState),
}

pub struct LoadedState {
    pub current_room: String,
    pub player_x: f32,
    pub player_y: f32,
    pub rando_data: RandoData,
    pub camera: Camera,
    pub hovered_room: Option<String>,
    pub selected_room: Option<String>,
    pub dragging_room: bool,
}

impl LoadedState {
    pub fn update(&mut self, map_data: &MapData) {
        self.camera.update(
            map_data,
            &self.rando_data,
            self.player_x,
            self.player_y,
            &self.current_room,
        );
    }
}

pub struct Camera {
    pub x: f32,
    pub y: f32,
    pub target: CameraTarget,
}

pub enum CameraTarget {
    Point { x: f32, y: f32 },
    Room(String),
    Player,
    PlayerRoom,
}

impl Camera {
    pub fn update(
        &mut self,
        map_data: &MapData,
        rando_data: &RandoData,
        player_x: f32,
        player_y: f32,
        current_room: &String,
    ) {
        let (tx, ty) = match &self.target {
            CameraTarget::Point { x, y } => (*x, *y),
            CameraTarget::Room(r) => {
                if let Some(pos) = rando_data.room_positions.get(r) {
                    *pos
                } else {
                    self.target = CameraTarget::PlayerRoom;
                    (0.0, 0.0)
                }
            },
            CameraTarget::Player => {
                if let Some(pos) = rando_data.room_positions.get(current_room) {
                    if let Some(room) = map_data.rooms.get(current_room) {
                        let bounds = room.calc_bounds();
                        (pos.0 + player_x, pos.1 - player_y + bounds.h)
                    } else {
                        (pos.0 + player_x, pos.1 - player_y)
                    }
                } else {
                    (player_x, player_y)
                }
            },
            CameraTarget::PlayerRoom => {
                if let Some(pos) = rando_data.room_positions.get(current_room) {
                    *pos
                } else {
                    (0.0, 0.0)
                }
            },
        };

        self.x += (tx - self.x) * 0.125;
        self.y += (ty - self.y) * 0.125;
    }
}

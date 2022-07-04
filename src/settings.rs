use egui::Ui;

pub struct Settings {
    pub depth: u8,
    pub draw_room_names: bool,
    pub debug_show_room_origins: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            depth: 2,
            draw_room_names: true,
            debug_show_room_origins: false,
        }
    }
}

impl Settings {
    pub fn fill_debug_egui(&mut self, ui: &mut Ui) {
        ui.add(egui::Slider::new(&mut self.depth, 0..=10).text("depth"));
        ui.checkbox(&mut self.draw_room_names, "draw_room_names");
        ui.checkbox(&mut self.debug_show_room_origins, "debug_show_room_origins");
    }
}

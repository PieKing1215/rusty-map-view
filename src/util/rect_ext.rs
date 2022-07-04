use ggez::graphics::Rect;

pub trait RectExt {
    fn inflate(&mut self, amount: f32);
}

impl RectExt for Rect {
    fn inflate(&mut self, amount: f32) {
        self.x -= amount;
        self.y -= amount;
        self.w += amount * 2.0;
        self.h += amount * 2.0;
    }
}

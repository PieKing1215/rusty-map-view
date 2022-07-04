use ggez::graphics::Color;

pub trait ColorExt {
    fn lerp(&self, other: &Self, thru: f32) -> Self;
}

impl ColorExt for Color {
    fn lerp(&self, other: &Self, thru: f32) -> Self {
        Color {
            r: lerp(self.r, other.r, thru),
            g: lerp(self.g, other.g, thru),
            b: lerp(self.b, other.b, thru),
            a: lerp(self.a, other.a, thru),
        }
    }
}

fn lerp(a: f32, b: f32, thru: f32) -> f32 {
    a + (b - a) * thru
}

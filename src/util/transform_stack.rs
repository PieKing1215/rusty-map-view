use ggez::graphics::DrawParam;
use nalgebra::{Matrix4, Point3, Vector3};

#[derive(Clone)]
pub struct TransformStack {
    stack: Vec<Matrix4<f32>>,
}

impl TransformStack {
    pub fn new() -> Self {
        TransformStack { stack: vec![Matrix4::identity()] }
    }

    pub fn push(&mut self) {
        self.stack.push(*self.stack.last().unwrap());
    }

    pub fn pop(&mut self) {
        self.stack.pop();
    }

    pub fn translate<T: Into<f64>>(&mut self, x: T, y: T) {
        *self.stack.last_mut().unwrap() = nalgebra_glm::translate(
            self.stack.last_mut().unwrap(),
            &nalgebra_glm::vec3(x.into() as f32, y.into() as f32, 0.0),
        );
    }

    pub fn scale<T: Into<f64>>(&mut self, x: T, y: T) {
        *self.stack.last_mut().unwrap() = nalgebra_glm::scale(
            self.stack.last_mut().unwrap(),
            &nalgebra_glm::vec3(x.into() as f32, y.into() as f32, 0.0),
        );
        // let prev_x = self.stack.last_mut().unwrap().scale_x;
        // let prev_y = self.stack.last_mut().unwrap().scale_y;

        // self.stack.last_mut().unwrap().scale_x *= x.into();
        // self.stack.last_mut().unwrap().scale_y *= y.into();
        // self.stack.last_mut().unwrap().translate_x /=
        //     self.stack.last_mut().unwrap().scale_x / prev_x;
        // self.stack.last_mut().unwrap().translate_y /=
        //     self.stack.last_mut().unwrap().scale_y / prev_y;
    }

    pub fn rotate<T: Into<f64>>(&mut self, angle: T) {
        *self.stack.last_mut().unwrap() = nalgebra_glm::rotate(
            self.stack.last_mut().unwrap(),
            angle.into() as f32,
            &Vector3::new(0.0, 0.0, 1.0),
        );
    }

    #[inline(always)]
    pub fn transform<T: Into<f64>>(&self, point: (T, T)) -> (f32, f32) {
        let t = self.stack.last().unwrap();
        let v = t.transform_point(&Point3::new(
            point.0.into() as f32,
            point.1.into() as f32,
            0.0,
        ));

        (
            v[0], v[1]
            // (point.0.into() + t.translate_x) * t.scale_x,
            // (point.1.into() + t.translate_y) * t.scale_y,
        )
    }

    #[inline(always)]
    pub fn transform_int<T: Into<f64>>(&self, point: (T, T)) -> (i32, i32) {
        let t = self.stack.last().unwrap();
        let v = t.transform_point(&Point3::new(
            point.0.into() as f32,
            point.1.into() as f32,
            0.0,
        ));

        (
            v[0] as i32, v[1] as i32
            // (point.0.into() + t.translate_x) * t.scale_x,
            // (point.1.into() + t.translate_y) * t.scale_y,
        )
    }

    #[allow(dead_code)]
    pub fn inv_transform<T: Into<f64>>(&self, point: (T, T)) -> (f32, f32) {
        let t = self.stack.last().unwrap();
        let v = t.try_inverse().unwrap().transform_point(&Point3::new(
            point.0.into() as f32,
            point.1.into() as f32,
            0.0,
        ));

        (
            v[0], v[1]
            // point.0.into() / t.scale_x - t.translate_x,
            // point.1.into() / t.scale_y - t.translate_y,
        )
    }

    #[allow(dead_code)]
    pub fn inv_transform_int<T: Into<f64>>(&self, point: (T, T)) -> (i32, i32) {
        let t = self.stack.last().unwrap();
        let v = t.try_inverse().unwrap().transform_point(&Point3::new(
            point.0.into() as f32,
            point.1.into() as f32,
            0.0,
        ));

        (
            v[0] as i32, v[1] as i32
            // (point.0.into() / t.scale_x - t.translate_x) as i32,
            // (point.1.into() / t.scale_y - t.translate_y) as i32,
        )
    }
}

impl Default for TransformStack {
    fn default() -> Self {
        Self::new()
    }
}

impl Into<DrawParam> for &TransformStack {
    fn into(self) -> DrawParam {
        let raw: [[f32; 4]; 4] = (*self.stack.last().unwrap()).into();
        let m: ggez::mint::ColumnMatrix4<f32> = ggez::mint::ColumnMatrix4::from(raw);
        DrawParam::default().transform(m)
    }
}

impl Into<DrawParam> for TransformStack {
    fn into(self) -> DrawParam {
        let raw: [[f32; 4]; 4] = (*self.stack.last().unwrap()).into();
        let m: ggez::mint::ColumnMatrix4<f32> = ggez::mint::ColumnMatrix4::from(raw);
        DrawParam::default().transform(m)
    }
}

use std::collections::HashMap;

use ggez::{graphics::{Rect, self, DrawParam, StrokeOptions, Color, Drawable, Font, PxScale}, GameResult};
use json::JsonValue;

use crate::{util::{transform_stack::TransformStack, color_ext::ColorExt}, settings::Settings};

use super::{item::Item, transition::Transition, RandoData};

pub struct Room {
    pub area: Option<String>,
    pub benches: Vec<(f32, f32)>,
    pub items: HashMap<String, Item>,
    pub name: Option<String>,
    pub randomizer_area: Option<String>,
    pub split_room: Option<Vec<Vec<String>>>,
    pub transitions: HashMap<String, Transition>,
}

impl Room {
    pub fn calc_bounds(&self) -> Rect {
        let mut min_x: f32 = 10000.0;
        let mut max_x: f32 = -10000.0;
        let mut min_y: f32 = 10000.0;
        let mut max_y: f32 = -10000.0;

        for (x, y) in &self.benches {
            min_x = min_x.min(*x - 5.0);
            max_x = max_x.max(*x + 5.0);
            min_y = min_y.min(-*y - 5.0);
            max_y = max_y.max(-*y + 5.0);
        }

        for (_, i) in &self.items {
            min_x = min_x.min(i.x - 5.0);
            max_x = max_x.max(i.x + 5.0);
            min_y = min_y.min(-i.y - 5.0);
            max_y = max_y.max(-i.y + 5.0);
        }

        for (_, t) in &self.transitions {
            min_x = min_x.min(t.x);
            max_x = max_x.max(t.x);
            min_y = min_y.min(-t.y);
            max_y = max_y.max(-t.y);
        }

        // add some padding if there was only 1 POI
        if (max_x - min_x) < 20.0 {
            min_x -= (20.0 - (max_x - min_x)) / 2.0;
            max_x += (20.0 - (max_x - min_x)) / 2.0;
        }

        if (max_y - min_y) < 20.0 {
            min_y -= (20.0 - (max_y - min_y)) / 2.0;
            max_y += (20.0 - (max_y - min_y)) / 2.0;
        }

        if min_x == 10000.0 {
            Rect { x: -10.0, y: -10.0, w: 20.0, h: 20.0 }
        } else {
            Rect { x: min_x, y: min_y, w: max_x - min_x, h: max_y - min_y }
        }
    }

    pub fn draw(&mut self, ctx: &mut ggez::Context, mut transform: TransformStack, key: &String, rando_data: &RandoData, asset_cache: &HashMap<String, graphics::Image>, hovered: bool, selected: bool, highlight_path: &Option<Vec<String>>, settings: &Settings) -> GameResult {
        let bounds = self.calc_bounds();

        if settings.debug_show_room_origins {
            let rect = graphics::Mesh::new_circle(
                ctx, 
                graphics::DrawMode::Stroke(StrokeOptions::default()), 
                [0.0, 0.0], 
                2.0,
                1.0,
                graphics::Color::from_rgba(255, 0, 0, 255)
            )?;

            graphics::draw(ctx, &rect, &transform)?;
        }


        transform.translate(0.0, bounds.h);

        let (stroke_color, fill_color) = match self.area.as_ref().map(|s| s.as_str()) {
            Some("Abyss") => (0xADACAD, 0x2D2D2D), //Ancient Basin
            Some("Cliffs") => (0x6B6B6B, 0x1B1B1B), //Howling Cliffs
            Some("Crossroads") => (0x9DC1DA, 0x2B353B), //Forgotten Crossroads
            Some("Deepnest") => (0x9AABC2, 0x262B30), //Deepnest
            Some("Deepnest_East") => (0xDFD1BE, 0x34312C), //Kingdom's Edge
            Some("FogCanyon") => (0xF3C8EB, 0x3C323A), //Fog Canyon
            Some("Fungus1") => (0xDCFFD0, 0x313D2E), //Greenpath
            Some("Fungus2") => (0xFAFFD3, 0x3B3D32), //Fungal Wastes
            Some("Fungus3") => (0x96B999, 0x232B23), //Queen's Gardens
            Some("Hive") => (0xFFF7A3, 0x3B3824), //The Hive
            Some("Mines") => (0xE4BEE8, 0x372F38), //Crystal Peak
            Some("RestingGrounds") => (0xFEC7A2, 0x382D24), //Resting Grounds
            Some("Room") => (0xFEA2AD, 0x382425), //Room
            Some("Ruins1") => (0xB8C3FF, 0x292C3C), //City of Tears
            Some("Ruins2") => (0xB8C3FF, 0x292C3C), //City of Tears
            Some("Town") => (0xA2A2A2, 0x2B2B2B), //Dirtmouth
            Some("Waterways") => (0x98FFFF, 0x243D3C), //Royal Waterways
            Some("White_Palace") => (0xD8D8D8, 0x333333), //White Palace
            _ => (0xA2A2A2, 0x2B2B2B),
        };

        let alpha = if selected {
            ((ggez::timer::time_since_start(ctx).as_secs_f32() / 0.33).sin().abs()) * 0.2 + 0.8
        } else if hovered {
            0.95
        } else {
            0.75
        };

        let mut path_highlight_factor = 0.0;
        if let Some(path) = highlight_path {
            if let Some(i) = path.iter().position(|path_tr| &Transition::get_transition_info(path_tr).unwrap().0 == key) {
                let thru = ((ggez::timer::time_since_start(ctx).as_secs_f32() + i as f32) / 0.25).sin().max(0.25);
                path_highlight_factor = thru;
            }
        }

        let rect = graphics::Mesh::new_rounded_rectangle(
            ctx, 
            graphics::DrawMode::fill(), 
            bounds, 
            5.0,
            graphics::Color::from_rgb_u32(fill_color).lerp(&graphics::Color::from_rgba(0, 0, 0, 0), 1.0 - alpha)
        )?;

        graphics::draw(ctx, &rect, &transform)?;

        let rect = graphics::Mesh::new_rounded_rectangle(
            ctx, 
            graphics::DrawMode::stroke(2.0),
            bounds, 
            5.0,
            graphics::Color::from_rgb_u32(stroke_color).lerp(&Color::from_rgb(255, 100, 160), path_highlight_factor).lerp(&graphics::Color::from_rgba(0, 0, 0, 0), 1.0 - alpha)
        )?;

        graphics::draw(ctx, &rect, &transform)?;

        if settings.debug_show_room_origins {
            let rect = graphics::Mesh::new_circle(
                ctx, 
                graphics::DrawMode::Stroke(StrokeOptions::default()), 
                [0.0, 0.0], 
                2.0,
                1.0,
                graphics::Color::from_rgba(0, 0, 255, 255)
            )?;

            graphics::draw(ctx, &rect, &transform)?;
        }
        
        
        let rect = graphics::Mesh::new_circle(
            ctx, 
            graphics::DrawMode::Stroke(StrokeOptions::default()), 
            [0.0, 0.0], 
            2.0,
            1.0,
            graphics::Color::from_rgba(0, 255, 0, 255)
        )?;

        // transitions
        
        let transition_normal = graphics::Mesh::new_polygon(
            ctx,
            graphics::DrawMode::fill(),
            &[
                [-1.0, -12.0],
                [1.0, -12.0],
                [6.0, 4.0],
                [-6.0, 4.0]
            ],
            graphics::Color::WHITE,
        )?;
        let transition_door = graphics::Mesh::new_polygon(
            ctx,
            graphics::DrawMode::fill(),
            &[
                [-6.0, 5.0],
                [-6.0, -8.0],
                [-4.0, -13.0],
                [0.0, -15.0],
                [4.0, -13.0],
                [6.0, -8.0],
                [6.0, 5.0],
            ],
            graphics::Color::WHITE,
        )?;
        for (n, tr) in &self.transitions {
            transform.push();
            transform.translate(tr.x, -tr.y);

            let transition_id = format!("{key}[{n}]");
            let revealed = rando_data.visited_transitions.contains(&transition_id);

            let mut color = if revealed {
                Color::from_rgba(150, 160, 150, 127)
            } else { 
                Color::from_rgba(255, 255, 127, 191).lerp(&Color::WHITE, ((ggez::timer::time_since_start(ctx).as_secs_f32()) / 0.5).sin().abs())
            };

            if let Some(path) = highlight_path {
                if let Some(i) = path.iter().position(|path_tr| path_tr == &transition_id) {
                    let thru = ((ggez::timer::time_since_start(ctx).as_secs_f32() + i as f32) / 0.25).sin().max(0.25);
                    color = color.lerp(&Color::from_rgb(255, 100, 160), thru);
                }
            }

            if n.starts_with("door") || n.starts_with("room") {
                let param: DrawParam = Into::<DrawParam>::into(&transform).color(color);
                graphics::draw(ctx, &transition_door, param)?;
                // graphics::draw(ctx, &rect, &transform);
            } else {
                if n.starts_with("left") {
                    transform.rotate(-90.0_f32.to_radians());
                } else if n.starts_with("right") {
                    transform.rotate(90.0_f32.to_radians());
                } else if n.starts_with("bot") {
                    transform.rotate(180.0_f32.to_radians());
                }
                let param: DrawParam = Into::<DrawParam>::into(&transform).color(color);
                graphics::draw(ctx, &transition_normal, param)?;
                // graphics::draw(ctx, &rect, &transform);
            }
            
            transform.pop();
        }

        // items

        let item = graphics::Mesh::new_circle(
            ctx,
            graphics::DrawMode::fill(),
            [0.0, 0.0],
            4.0,
            2.0,
            graphics::Color::YELLOW,
        )?;
        for (_, i) in &self.items {
            transform.push();
            transform.translate(i.x, -i.y);

            graphics::draw(ctx, &item, &transform)?;

            transform.pop();
        }

        // benches

        if let Some(img) = asset_cache.get("pin_bench") {
            let bench = graphics::Mesh::new_rectangle(
                ctx,
                graphics::DrawMode::fill(),
                Rect::new(-10.0, -4.0 + 3.0, 20.0, 8.0),
                graphics::Color::CYAN,
            )?;
            for (x, y) in &self.benches {
                transform.push();
                transform.translate(*x, -*y);

                transform.scale(0.33, 0.33);
                transform.translate(-(img.width() as f32) / 2.0, -(img.height() as f32) / 2.0);
    
                graphics::draw(ctx, img, &transform)?;
    
                transform.pop();
            }
        } else {
            let bench = graphics::Mesh::new_rectangle(
                ctx,
                graphics::DrawMode::fill(),
                Rect::new(-10.0, -4.0 + 3.0, 20.0, 8.0),
                graphics::Color::CYAN,
            )?;
            for (x, y) in &self.benches {
                transform.push();
                transform.translate(*x, -*y);
    
                graphics::draw(ctx, &bench, &transform)?;
                // graphics::draw(ctx, &rect, &transform);
    
                transform.pop();
            }
        }

        // room name
        if settings.draw_room_names {
            transform.push();
            transform.translate(bounds.x, bounds.y + bounds.h);
            // TODO: cache
            graphics::Text::new(key.clone()).set_font(Font::default(), PxScale::from(12.0)).draw(ctx, (&transform).into())?;
            transform.pop();
        }

        Ok(())
    }
}

impl TryFrom<&JsonValue> for Room {
    type Error = String;

    fn try_from(json: &JsonValue) -> Result<Self, Self::Error> {

        let area = json["area"].as_str().map(|s| s.into());

        let benches = json["benches"].members().map(|v| {
            Ok((v["x"].as_f32().ok_or("Bench has missing/invalid field 'x'")?, v["y"].as_f32().ok_or("Bench has missing/invalid field 'y'")?))
        }).collect::<Result<_, Self::Error>>()?;

        let items = json["items"].entries().map(|(k, v)| {
            Ok((k.into(), v.try_into()?))
        }).collect::<Result<HashMap<String, Item>, Self::Error>>()?;

        let name = json["name"].as_str().map(|s| s.into());
        let randomizer_area = json["randomizerArea"].as_str().map(|s| s.into());

        let split_room = if json["splitRoom"].is_array() {
            Some(json["splitRoom"].members().map(|m| {
                m.members().map(|tr| tr.as_str().ok_or("Room has invalid splitRoom entry").map(|s| s.into())).collect()
            }).collect::<Result<Vec<_>, _>>()?)
        } else {
            None
        };

        let transitions = json["transitions"].entries().map(|(k, v)| {
            Ok((k.into(), v.try_into()?))
        }).collect::<Result<HashMap<String, Transition>, Self::Error>>()?;

        Ok(Self {
            area,
            benches,
            items,
            name,
            randomizer_area,
            split_room,
            transitions,
        })
    }
}

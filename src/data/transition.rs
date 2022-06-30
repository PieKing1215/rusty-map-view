use json::JsonValue;


pub struct Transition {
    pub to: Option<String>,
    pub x: f32,
    pub y: f32,
}

impl Transition {
    pub fn get_transition_info(transition: &String) -> Option<(String, String)> {
        transition.split_once("[").map(|t| (t.0.into(), t.1.trim_end_matches("]").into()))
    }
}

impl TryFrom<&JsonValue> for Transition {
    type Error = String;

    fn try_from(json: &JsonValue) -> Result<Self, Self::Error> {
        
        Ok(Self {
            to: json["to"].as_str().map(|s| s.into()),
            x: json["x"].as_f32().unwrap_or(0.0),
            y: json["y"].as_f32().unwrap_or(0.0),
        })
    }
}
use json::JsonValue;


pub struct Item {
    pub x: f32,
    pub y: f32,
    //geo
    //randAction
    pub rand_pool: String,
    //randType
}

impl TryFrom<&JsonValue> for Item {
    type Error = String;

    fn try_from(json: &JsonValue) -> Result<Self, Self::Error> {
        Ok(Self {
            x: json["x"].as_f32().unwrap_or(0.0),
            y: json["y"].as_f32().unwrap_or(0.0),
            rand_pool: json["randPool"].as_str().unwrap().into(),
        })
    }
}
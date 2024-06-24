use std::str::FromStr;

use jsonpath::JsonPathInst;

pub struct JsonPath(pub JsonPathInst);

impl<'de> serde::Deserialize<'de> for JsonPath {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match JsonPathInst::from_str(<&str>::deserialize(deserializer)?) {
            Ok(value) => Ok(Self(value)),
            Err(error) => Err(<D::Error as serde::de::Error>::custom(error)),
        }
    }
}

#[derive(Deserialize)]
pub struct API {
    pub path: String,
    #[serde(rename = "entity")]
    pub is_entity: Option<bool>,
    pub jsonpath: Option<JsonPath>,
    pub apis: Option<Vec<API>>,
}

#[derive(Deserialize)]
pub struct Config {
    pub url: String,
    pub apis: Vec<API>,
}

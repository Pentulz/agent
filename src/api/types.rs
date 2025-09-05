use reqwest::StatusCode;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

// Struct to map API JSON successful responses
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiData<T> {
    #[serde(
        serialize_with = "serialize_status_code",
        deserialize_with = "deserialize_status_code"
    )]
    pub code: Option<StatusCode>,
    pub data: Option<T>,
}

impl<T> ApiData<T> {
    pub fn new() -> ApiData<T> {
        ApiData {
            data: None,
            code: None,
        }
    }
}

// JSON serialization / deserialization methods
fn serialize_status_code<S>(code: &Option<StatusCode>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match code {
        Some(status) => serializer.serialize_some(&status.as_u16()),
        None => serializer.serialize_none(),
    }
}

fn deserialize_status_code<'de, D>(deserializer: D) -> Result<Option<StatusCode>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt: Option<u16> = Option::deserialize(deserializer)?;
    match opt {
        Some(code) => StatusCode::from_u16(code)
            .map(Some)
            .map_err(serde::de::Error::custom),
        None => Ok(None),
    }
}

use core::fmt;
use std::fmt::Display;

use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_repr::Deserialize_repr;

#[derive(Debug, Serialize, Deserialize)]
pub struct Sticker {
    #[serde(rename(deserialize = "stickerId"))]
    sticker_id: u32,
    slot: u8,
    codename: String,
    material: String,
    name: String,
}

#[derive(Serialize, Deserialize)]
struct FloatItemResponse {
    pub iteminfo: ItemDescription,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ItemDescription {
    origin: u32,
    quality: u32,
    rarity: u32,
    a: String,
    d: String,
    #[serde(rename(deserialize = "paintseed"))]
    paint_seed: u32,
    #[serde(rename(deserialize = "defindex"))]
    def_index: u32,
    stickers: Vec<Sticker>,
    #[serde(rename(deserialize = "floatid"))]
    float_id: String,
    #[serde(rename(deserialize = "floatvalue"))]
    float_value: f32,
    s: String,
    m: String,
    #[serde(rename(deserialize = "imageurl"))]
    image_url: String,
    min: f32,
    max: f32,
    weapon_type: String,
    item_name: String,
    rarity_name: String,
    quality_name: String,
    origin_name: String,
    wear_name: String,
    full_item_name: String,
}

#[derive(Debug)]
pub struct CsgoFloatError {
    code: CsgoFloatErrorCode,
}

impl Display for CsgoFloatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CSGOFloat request failed: {}", self.code)
    }
}

#[derive(Deserialize_repr, Debug)]
#[repr(u8)]
pub enum CsgoFloatErrorCode {
    ImproperParameterStructure = 1,
    InvalidInspectLinkStructure = 2,
    TooManyPendingRequests = 3,
    ValveServerTimeout = 4,
    ValveOffline = 5,
    CsgoFloatInternalError = 6,
    ImproperBodyFormat = 7,
    BadSecret = 8,
}

impl Display for CsgoFloatErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ImproperParameterStructure => write!(f, "Improper parameter structure"),
            Self::InvalidInspectLinkStructure => write!(f, "Invalid Inspect Link Structure"),
            Self::TooManyPendingRequests => {
                write!(f, "You have too many pending requests open at once")
            }
            Self::ValveServerTimeout => write!(f, "Valve's servers didn't reply in time"),
            Self::ValveOffline => write!(
                f,
                "Valve's servers appear to be offline, please try again later"
            ),
            Self::CsgoFloatInternalError => {
                write!(f, "Something went wrong on our end, please try again")
            }
            Self::ImproperBodyFormat => write!(f, "Improper body format"),
            Self::BadSecret => write!(f, "Bad Secret"),
        }
    }
}

#[derive(Debug)]
pub enum CsgoFloatFetchError {
    CsgoFloat(CsgoFloatError),
    Transport(reqwest::Error),
    Deserializing,
}

impl From<reqwest::Error> for CsgoFloatFetchError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_decode() {
            eprintln!("Error deserializing JSON response: {}", e);
            Self::Deserializing
        } else {
            Self::Transport(e)
        }
    }
}

pub async fn get_by_market_url(
    client: &Client,
    market_url: &str,
) -> Result<ItemDescription, CsgoFloatFetchError> {
    let url = format!("https://api.csgofloat.com?url={}", market_url);
    let resp = client.get(url).send().await?;

    match resp.status() {
        StatusCode::OK => {
            let data: FloatItemResponse = resp.json().await?;
            Ok(data.iteminfo)
        },
        status => {
            eprintln!("CSGOFloat responded with error status {}", status);
            resp.json().await.map_err(|e| e.into())
        }
    }
}

use reqwest::StatusCode;
use thiserror::Error;

use super::parsing::{AuthenticationParseError, ParseFailure};

// TODO: Handle rate limiting errors as its' own special category??

#[derive(Debug, Error)]
pub enum FetchInventoryError {
    #[error("HTTP error: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("error deserlializing inventory: {0}")]
    Deserializing(#[from] serde_json::Error),
}

#[derive(Debug, Error)]
pub enum FetchNewUnpreparedItemsError {
    #[error("HTTP error: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("Authentication failure")]
    Authentication,
    #[error("unhandled status code: {0}")]
    UnhandledStatusCode(StatusCode),
    #[error("failed to parse any history from steam site")]
    NoHistoryFound,
    #[error("failed to parse inventory history: {0}")]
    PageParse(#[from] ParseFailure),
    #[error("error finding login state: {0}")]
    AuthenticationParse(#[from] AuthenticationParseError),
    #[error("not logged in")]
    NotAuthenticated,
}

#[derive(Debug, Error)]
pub enum FetchItemsError {
    #[error("error fetching inventory: {0}")]
    FetchInventory(#[from] FetchInventoryError),
    #[error("error fetching raw items: {0}")]
    FetchUnpreparedItems(#[from] FetchNewUnpreparedItemsError),
}

#[derive(Debug, Error)]
pub enum PrepareItemsError {
    #[error("http error fetching inventory: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("error deserialising inventory: {0}")]
    Deserializing(#[from] serde_json::Error),
    #[error("error fetching inventory data: {0}")]
    PreparingItems(#[from] FetchItemsError),
}

#[derive(Debug, Error)]
pub enum MarketPriceFetchError {
    #[error("http error: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("error deserialising market prices: {0}")]
    Deserializing(#[from] serde_json::Error),
}

#[derive(Debug, Error)]
pub enum AuthenticationCheckError {
    #[error("http error: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("error parsing authentication data: {0}")]
    Parse(#[from] AuthenticationParseError),
}

#[derive(Debug, Error)]
pub enum LocalPrepareError {
    #[error("could not find item description in inventory")]
    NoDescription,
    #[error("could not find item asset info in inventory")]
    NoAsset,
    #[error("could not find in-game inspect link")]
    NoInspectLink,
}

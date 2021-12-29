use std::fmt::{self, Display};

use hyper::StatusCode;

use super::parsing::{AuthenticationParseError, ParseFailure};

#[derive(Debug)]
pub enum FetchNewUnpreparedItemsError {
    Transport(reqwest::Error),
    Authentication,
    UnhandledStatusCode(StatusCode),
    NoHistoryFound,
    PageParse(ParseFailure),
    AuthenticationParse(AuthenticationParseError),
    NotAuthenticated,
}

impl From<ParseFailure> for FetchNewUnpreparedItemsError {
    fn from(e: ParseFailure) -> Self {
        Self::PageParse(e)
    }
}

impl From<AuthenticationParseError> for FetchNewUnpreparedItemsError {
    fn from(e: AuthenticationParseError) -> Self {
        Self::AuthenticationParse(e)
    }
}

impl std::fmt::Display for FetchNewUnpreparedItemsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport(e) => write!(f, "HTTP error: {}", e),
            Self::Authentication => write!(f, "Authentication failure"),
            Self::UnhandledStatusCode(code) => write!(f, "unhandled status code: {}", code),
            Self::NoHistoryFound => write!(f, "failed to parse any history from steam site"),
            Self::PageParse(e) => write!(f, "failed to parse inventory history: {}", e),
            Self::AuthenticationParse(e) => write!(f, "error finding login state: {}", e),
            Self::NotAuthenticated => write!(f, "not logged in"),
        }
    }
}

impl From<reqwest::Error> for FetchNewUnpreparedItemsError {
    fn from(e: reqwest::Error) -> Self {
        Self::Transport(e)
    }
}

#[derive(Debug)]
pub enum FetchItemsError {
    FetchUnpreparedItems(FetchNewUnpreparedItemsError),
    PreparingItems(PrepareItemsError),
}

impl From<FetchNewUnpreparedItemsError> for FetchItemsError {
    fn from(e: FetchNewUnpreparedItemsError) -> Self {
        Self::FetchUnpreparedItems(e)
    }
}

impl From<PrepareItemsError> for FetchItemsError {
    fn from(e: PrepareItemsError) -> Self {
        Self::PreparingItems(e)
    }
}

impl Display for FetchItemsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FetchUnpreparedItems(e) => write!(f, "error fetching raw items: {}", e),
            Self::PreparingItems(e) => {
                write!(f, "error preparing items from inventory data: {}", e)
            }
        }
    }
}

#[derive(Debug)]
pub enum PrepareItemsError {
    Transport(reqwest::Error),
    Deserializing(serde_json::Error),
}

impl From<reqwest::Error> for PrepareItemsError {
    fn from(e: reqwest::Error) -> Self {
        Self::Transport(e)
    }
}

impl From<serde_json::Error> for PrepareItemsError {
    fn from(e: serde_json::Error) -> Self {
        Self::Deserializing(e)
    }
}

impl Display for PrepareItemsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transport(e) => write!(f, "http error fetching inventory: {}", e),
            Self::Deserializing(e) => write!(f, "error deserialising inventory: {}", e),
        }
    }
}

#[derive(Debug)]
pub enum MarketPriceFetchError {
    Transport(reqwest::Error),
    Deserializing(serde_json::Error),
}

impl Display for MarketPriceFetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transport(e) => write!(f, "http error: {}", e),
            Self::Deserializing(e) => write!(f, "error deserialising market prices: {}", e),
        }
    }
}

impl From<reqwest::Error> for MarketPriceFetchError {
    fn from(e: reqwest::Error) -> Self {
        Self::Transport(e)
    }
}

impl From<serde_json::Error> for MarketPriceFetchError {
    fn from(e: serde_json::Error) -> Self {
        Self::Deserializing(e)
    }
}

#[derive(Debug)]
pub enum AuthenticationCheckError {
    Transport(reqwest::Error),
    Parse(AuthenticationParseError),
}

impl Display for AuthenticationCheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transport(e) => write!(f, "http error: {}", e),
            Self::Parse(e) => write!(f, "error parsing authentication data: {}", e),
        }
    }
}

impl From<reqwest::Error> for AuthenticationCheckError {
    fn from(e: reqwest::Error) -> Self {
        Self::Transport(e)
    }
}

impl From<AuthenticationParseError> for AuthenticationCheckError {
    fn from(e: AuthenticationParseError) -> Self {
        Self::Parse(e)
    }
}

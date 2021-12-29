use std::fmt::{self, Display};

use hyper::StatusCode;

use super::parsing::{AuthenticationParseError, ParseFailure};

#[derive(Debug)]
pub enum FetchNewUnpreparedItemsError {
    TransportError(reqwest::Error),
    AuthenticationFailure,
    UnhandledStatusCode(StatusCode),
    NoHistoryFound,
    PageParseError(ParseFailure),
    AuthenticationParseError(AuthenticationParseError),
    NotAuthenticated,
}

impl From<ParseFailure> for FetchNewUnpreparedItemsError {
    fn from(e: ParseFailure) -> Self {
        Self::PageParseError(e)
    }
}

impl From<AuthenticationParseError> for FetchNewUnpreparedItemsError {
    fn from(e: AuthenticationParseError) -> Self {
        Self::AuthenticationParseError(e)
    }
}

impl std::fmt::Display for FetchNewUnpreparedItemsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TransportError(e) => write!(f, "HTTP error: {}", e),
            Self::AuthenticationFailure => write!(f, "Authentication failure"),
            Self::UnhandledStatusCode(code) => write!(f, "unhandled status code: {}", code),
            Self::NoHistoryFound => write!(f, "failed to parse any history from steam site"),
            Self::PageParseError(e) => write!(f, "failed to parse inventory history: {}", e),
            Self::AuthenticationParseError(e) => write!(f, "error finding login state: {}", e),
            Self::NotAuthenticated => write!(f, "not logged in"),
        }
    }
}

impl From<reqwest::Error> for FetchNewUnpreparedItemsError {
    fn from(e: reqwest::Error) -> Self {
        Self::TransportError(e)
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

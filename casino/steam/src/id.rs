use regex::Regex;
use reqwest::Url;
use scraper::Html;
use thiserror::Error;

use super::parsing::{get_userid, UserIdParseError};

lazy_static::lazy_static! {
    static ref PROFILE_URL_REGEX: Regex = Regex::new(r#"steamcommunity\.com/(?:id/([a-zA-Z0-9-_]+)|profiles/([0-9]+))"#).unwrap();
}

#[derive(Clone)]
pub struct Id {
    id: u64,
    vanity: Option<String>,

    profile_url: Url,
    inventory_url: Url,
    inventory_history_url: Url,
}

impl Id {
    pub fn new(id: u64, vanity: Option<String>) -> Self {
        let profile_url = match vanity.as_deref() {
            Some(v) => format_profile_url_vanity(v),
            None => format_profile_url_id(id),
        };

        let inventory_url = format_inventory_url(id);
        let inventory_history_url = format_inventory_history_url(profile_url.as_str());

        Self {
            id,
            vanity,

            profile_url,
            inventory_url,
            inventory_history_url,
        }
    }

    pub async fn try_from_url(url_ish: &str) -> Result<Self, IdUrlParseError> {
        let url_match = parse_profile_url(url_ish).ok_or(IdUrlParseError::InvalidProfileUrl)?;
        let url = match &url_match {
            ProfileUrlMatch::SteamId(id) => format_profile_url_id(*id),
            ProfileUrlMatch::VanityUrl(v) => format_profile_url_vanity(v),
        };
        let resp = reqwest::get(url.as_ref()).await?;
        let profile_url = resp.url().to_owned();
        if profile_url != url {
            // Should we do something different if we're given a by-id-only
            // url, and are redirected to a vanity url?
            log::warn!(
                "redirected from given url {} to canonical url {}",
                url,
                profile_url
            );
        }
        let resp_data = resp.text().await?;
        let parsed = Html::parse_document(&resp_data);
        let id = get_userid(&parsed)?;
        let vanity = match url_match {
            ProfileUrlMatch::SteamId(_) => None,
            ProfileUrlMatch::VanityUrl(v) => Some(v),
        };
        let inventory_url = format_inventory_url(id);
        let inventory_history_url = format_inventory_history_url(profile_url.as_str());

        Ok(Self {
            id,
            vanity,

            profile_url,
            inventory_url,
            inventory_history_url,
        })
    }

    pub fn user_id(&self) -> u64 {
        self.id
    }

    pub fn inventory_url(&self) -> &str {
        self.inventory_url.as_ref()
    }

    pub fn profile_url(&self) -> &str {
        self.profile_url.as_ref()
    }

    pub fn inventory_history_url(&self) -> &str {
        self.inventory_history_url.as_ref()
    }
}

#[derive(Debug, Error)]
pub enum IdUrlParseError {
    #[error("invalid steam profile url")]
    InvalidProfileUrl,
    #[error("http error: {0}")]
    TransportError(#[from] reqwest::Error),
    #[error("error parsing user information: {0}")]
    ValidationError(#[from] UserIdParseError),
}

#[derive(Debug, PartialEq, Eq)]
enum ProfileUrlMatch {
    VanityUrl(String),
    SteamId(u64),
}

fn parse_profile_url(url: &str) -> Option<ProfileUrlMatch> {
    let matches = PROFILE_URL_REGEX.captures(url)?;

    if let Some(m) = matches.get(1) {
        Some(ProfileUrlMatch::VanityUrl(m.as_str().to_string()))
    } else if let Some(m) = matches.get(2) {
        // validity should be guaranteed by regex
        let steam_id: u64 = m.as_str().parse().unwrap();
        Some(ProfileUrlMatch::SteamId(steam_id))
    } else {
        None
    }
}

fn format_profile_url_id(id: u64) -> Url {
    format!("https://steamcommunity.com/profiles/{}", id)
        .parse()
        .unwrap()
}

fn format_profile_url_vanity(vanity: &str) -> Url {
    format!("https://steamcommunity.com/id/{}", vanity)
        .parse()
        .unwrap()
}

fn format_inventory_url(id: u64) -> Url {
    format!(
        "https://steamcommunity.com/inventory/{}/730/2?l=english&count=25",
        id
    )
    .parse()
    .unwrap()
}

fn format_inventory_history_url(base: &str) -> Url {
    format!("{}/inventoryhistory/?app[]=730", base)
        .parse()
        .unwrap()
}

#[cfg(test)]
mod test {
    use super::{parse_profile_url, ProfileUrlMatch};

    #[test]
    fn test_profile_url_parsing_vanity() {
        let by_vanity = "https://steamcommunity.com/id/badcop_";
        let parsed = parse_profile_url(by_vanity).unwrap();
        let expected = ProfileUrlMatch::VanityUrl("badcop_".to_string());

        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_profile_url_parsing_steamid() {
        let by_steamid = "https://steamcommunity.com/profiles/76561198000494793";
        let parsed = parse_profile_url(by_steamid).unwrap();
        let expected = ProfileUrlMatch::SteamId(76561198000494793);

        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_profile_url_parsing_error() {
        let by_steamid = "https://steamcommunity.com/profiles/abc123";
        assert!(parse_profile_url(by_steamid).is_none());
    }
}

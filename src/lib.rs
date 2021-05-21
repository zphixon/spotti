use std::{net::SocketAddr, path::PathBuf};
use url::Url;

pub const SPOTIFY_AUTH_URL: &'static str =
    "https://accounts.spotify.com/authorize?response_type=code";
pub const SPOTIFY_TOKEN_URL: &'static str = "https://accounts.spotify.com/api/token";
pub const ME: &str = "th59jhhlgloqhkwcj5foha869";

pub struct SongRecord {
    pub name: Option<String>,
    pub album: Option<String>,
    pub artist: Option<String>,
    pub date: Option<String>,
    pub id: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Listens {
    pub items: Vec<Listen>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Listen {
    pub played_at: String,
    pub track: Track,
}

#[derive(Debug, serde::Deserialize)]
pub struct Track {
    pub album: Album,
    pub artists: Vec<Artist>,
    pub name: String,
    pub r#type: String,
    pub id: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct Album {
    pub album_type: String,
    pub artists: Vec<Artist>,
    pub name: String,
    pub r#type: String,
    pub id: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct Artist {
    pub name: String,
    pub id: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Me {
    pub id: String,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct MaybeAuth {
    pub access_token: String,
    pub refresh_token: Option<String>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct SessionAuth(pub TokenPair);

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct GlobalAuth(pub TokenPair);

#[derive(Debug, serde::Deserialize)]
pub struct StringConfig {
    db_file: String,
    error_file: String,
    bot_pidfile: String,

    client_id: String,
    client_secret: String,

    base_url: String,
    authorize_endpoint: String,
    refresh_endpoint: String,
    get_new_endpoint: String,
    show_all_endpoint: String,
    uptime_endpoint: String,

    get_new_limit: u32,

    address: String,
}

#[derive(Debug)]
pub struct Config {
    pub db_file: String,
    pub error_file: PathBuf,
    pub bot_pidfile: PathBuf,
    pub client_id: String,
    pub client_secret: String,

    pub authorize_url: Url,
    pub refresh_url: Url,
    pub get_new_url: Url,
    pub show_all_url: Url,
    pub uptime_url: Url,

    pub get_new_limit: u32,

    pub address: SocketAddr,
}

pub fn make_link(href: &str, text: &str) -> String {
    format!("<a href={href}>{text}</a>")
}

impl Config {
    pub fn authorize_link(&self, text: &str) -> String {
        make_link(&self.authorize_url.as_str(), text)
    }

    pub fn refresh_link(&self, text: &str) -> String {
        make_link(&self.refresh_url.as_str(), text)
    }

    pub fn get_new_link(&self, text: &str) -> String {
        make_link(&self.get_new_url.as_str(), text)
    }

    pub fn show_all_link(&self, text: &str) -> String {
        make_link(&self.show_all_url.as_str(), text)
    }
}

impl From<StringConfig> for Config {
    fn from(config: StringConfig) -> Config {
        let db_file = PathBuf::from(&config.db_file);
        let error_file = PathBuf::from(&config.error_file);
        let bot_pidfile = PathBuf::from(&config.bot_pidfile);
        assert!(
            db_file.exists(),
            "db file {} does not exist",
            db_file.display()
        );

        let base_url = Url::parse(&config.base_url).expect("invalid base URL");
        assert!(base_url.domain().is_some(), "need domain name");

        let mut authorize_url = base_url.clone();
        if config.authorize_endpoint != "" {
            authorize_url
                .path_segments_mut()
                .unwrap()
                .push(&config.authorize_endpoint);
        }

        let mut refresh_url = base_url.clone();
        if config.refresh_endpoint != "" {
            refresh_url
                .path_segments_mut()
                .unwrap()
                .push(&config.refresh_endpoint);
        }

        let mut get_new_url = base_url.clone();
        if config.get_new_endpoint != "" {
            get_new_url
                .path_segments_mut()
                .unwrap()
                .push(&config.get_new_endpoint);
        }

        let mut show_all_url = base_url.clone();
        if config.show_all_endpoint != "" {
            show_all_url
                .path_segments_mut()
                .unwrap()
                .push(&config.show_all_endpoint);
        }

        let mut uptime_url = base_url.clone();
        if config.uptime_endpoint != "" {
            uptime_url
                .path_segments_mut()
                .unwrap()
                .push(&config.uptime_endpoint);
        }

        let address = config
            .address
            .parse::<SocketAddr>()
            .expect("invalid address");

        tracing::info!("{}", authorize_url.as_str());
        tracing::info!("{}", get_new_url.as_str());
        tracing::info!("{}", refresh_url.as_str());
        tracing::info!("{}", show_all_url.as_str());
        tracing::info!("{}", uptime_url.as_str());
        Config {
            db_file: config.db_file,
            error_file,
            bot_pidfile,

            client_id: config.client_id,
            client_secret: config.client_secret,

            authorize_url,
            refresh_url,
            get_new_url,
            show_all_url,
            uptime_url,

            get_new_limit: config.get_new_limit,

            address,
        }
    }
}

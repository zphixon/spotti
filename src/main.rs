use axum::{
    extract,
    headers::AccessControlAllowOrigin,
    http::StatusCode,
    response,
    response::{Html, IntoResponse, Result},
    routing, TypedHeader,
};
use axum_sessions::{
    async_session::{
        self,
        chrono::{DateTime, Utc},
    },
    extractors::WritableSession,
    SessionLayer,
};
use once_cell::sync::Lazy;
use rand::RngCore;
use reqwest as request;
use spotti::{Config, GlobalAuth, Listens, Me, SessionAuth, SongRecord, StringConfig, TokenPair};
use std::{net::SocketAddr, sync::RwLock};
use url::Url;

fn unauthorized() -> response::Response {
    (
        StatusCode::BAD_REQUEST,
        Html(format!(
            r#"<!doctype html>
<html>
  <head><title>unauthorized</title></head>
  <body>
    <h1>you're unauthorized</h1>
    <p>go get {}</p>
  </body>
</html>"#,
            CONFIG.authorize_link("authorized")
        )),
    )
        .into_response()
}

async fn not_found(extract::OriginalUri(path): extract::OriginalUri) -> response::Response {
    (
        StatusCode::NOT_FOUND,
        Html(format!(
            r#"<!doctype html>
<html>
  <head><title>uhhhh</title></head>
  <body>
    <h1>uhhhh</h1>
    <p>you DEFINITELY shouldn't be able to see this</p>
    <p>{}</p>
    <p>you requested "{}"</p>
  </body>
</html>"#,
            CONFIG.get_new_link("try this?"),
            path
        )),
    )
        .into_response()
}

static CONFIG: Lazy<Config> = Lazy::new(|| {
    let filename = std::env::args()
        .nth(1)
        .expect("missing config file cmd line argument");
    let contents = std::fs::read_to_string(filename).expect("couldn't read config file");

    let string_config: StringConfig = toml::from_str(&contents).unwrap();
    let config = Config::from(string_config);
    tracing::debug!("{config:#?}");
    config
});

static GLOBAL_AUTH: Lazy<RwLock<Option<GlobalAuth>>> = Lazy::new(|| RwLock::new(None));

static START_TIME: Lazy<DateTime<Utc>> = Lazy::new(|| Utc::now());

const PAGE_HEADER: &str = r#"
<!doctype html>
<head><title>NOT LAST.FM</title></head>
<style>
table, td, th {
    border: 1px solid #090;
    border-collapse: collapse;
    padding-left: 4pt;
    padding-right: 8pt;
}
.datetime {
    width: 20%;
}
</style>
<body>
<h1>what's zack been listening to recently?</h1>
"#;

const PAGE_FOOTER: &str = "</body></html>";

macro_rules! five_hundred {
    ($why:literal) => {
        five_hundred!($why, "xd lmao")
    };

    ($why:literal, $more:tt) => {
        |err| {
            let err = err.to_string();
            let _ = std::fs::write(&CONFIG.error_file, &err);
            if let Ok(pid) = std::fs::read_to_string(&CONFIG.bot_pidfile) {
                let _ = std::process::Command::new("kill")
                    .arg("-usr1")
                    .arg(&pid)
                    .output();
            }

            let smore = match format!("{:?}", $more) {
                xd if xd == r#""xd lmao""# => None,
                not_xd => Some(not_xd),
            };

            tracing::error!("{err} {:?}", smore);

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(format!(
                    r#"<!doctype html>
<html>
  <head><title>NOT LAST.FM: 500</title></head>
  <body>
    <h1>500 internal server error</h1>
    <p>{}</p>
    <pre><code>{}</code></pre>
    <p>try {} or {}. if the problem persists tell zack</p>{}
  </body>
</html>"#,
                    $why,
                    err,
                    CONFIG.authorize_link("authing"),
                    CONFIG.refresh_link("refreshing"),
                    if let Some(more) = smore {
                        format!(
                            r#"
    <p>more info:</p>
    <pre style=white-space:pre-wrap;><code>{}</code></pre>"#,
                            more.replace("&", "&amp;")
                                .replace("<", "&lt;")
                                .replace(">", "&gt;")
                        )
                    } else {
                        String::new()
                    }
                )),
            )
                .into_response()
        }
    };
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("starting {:?}", *START_TIME);

    let mut secret = [0; 512];
    rand::thread_rng().fill_bytes(&mut secret);
    let store = async_session::MemoryStore::new();
    let session_layer = SessionLayer::new(store, &secret);

    let app = axum::Router::new()
        .route(&CONFIG.get_new_url.path(), routing::get(get_new))
        .route(&CONFIG.show_all_url.path(), routing::get(show_all))
        .route(&CONFIG.authorize_url.path(), routing::get(authorize))
        .route(&CONFIG.refresh_url.path(), routing::get(refresh))
        .route(&CONFIG.uptime_url.path(), routing::get(uptime))
        .layer(session_layer);

    let app = app.fallback(not_found);

    tracing::info!("listening on {:?}", &CONFIG.address);
    axum::Server::bind(&CONFIG.address)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();
}

async fn get_new(session: WritableSession) -> Result<response::Response> {
    do_db_stuff(session, Some(CONFIG.get_new_limit)).await
}

async fn show_all(session: WritableSession) -> Result<response::Response> {
    do_db_stuff(session, None).await
}

async fn authorize(
    extract::Query(query): extract::Query<std::collections::HashMap<String, String>>,
    extract::ConnectInfo(addr): extract::ConnectInfo<SocketAddr>,
    mut session: WritableSession,
) -> Result<response::Response> {
    if let Some(code) = query.get("code") {
        tracing::debug!("{addr} got code, doing oauth2");
        tracing::trace!("code={}", code);
        return do_oauth2(code, &mut session).await;
    }

    let spotify_auth_redirect = Url::parse_with_params(
        spotti::SPOTIFY_AUTH_URL,
        &[
            ("client_id", &CONFIG.client_id),
            ("redirect_uri", &String::from(CONFIG.authorize_url.as_str())),
            (
                "scope",
                &String::from("user-read-recently-played user-modify-playback-state"),
            ),
        ],
    )
    .map_err(five_hundred!("spotify_auth_redirect malformed"))?;

    tracing::debug!("{addr} redirecting to {}", spotify_auth_redirect.as_str());
    Ok(response::Redirect::to(spotify_auth_redirect.as_str()).into_response())
}

async fn refresh() -> Result<response::Response> {
    let refresh_url = {
        let Some(auth) = &*GLOBAL_AUTH.read().map_err(five_hundred!("lock global auth refresh read"))? else {
        return Ok(unauthorized());
    };

        Url::parse_with_params(
            spotti::SPOTIFY_TOKEN_URL,
            &[
                ("grant_type", "refresh_token"),
                ("refresh_token", &auth.0.refresh_token),
                ("redirect_uri", CONFIG.authorize_url.as_str()),
                ("client_id", &CONFIG.client_id),
                ("client_secret", &CONFIG.client_secret),
            ],
        )
        .map_err(five_hundred!("refresh url malformed"))?
    };

    let client = reqwest::Client::new();
    let response = client
        .post(refresh_url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Content-Length", "0")
        .send()
        .await
        .map_err(five_hundred!("refresh request"))?
        .text()
        .await
        .map_err(five_hundred!("refresh content"))?;

    tracing::debug!("refresh: {:?}", response);
    let maybe_auth: spotti::MaybeAuth =
        serde_json::from_str(&response).map_err(five_hundred!("refresh json"))?;

    let mut global_auth = GLOBAL_AUTH
        .write()
        .map_err(five_hundred!("lock global auth refresh write"))?;

    let auth = global_auth.as_mut().unwrap();

    auth.0.access_token = maybe_auth.access_token;
    if let Some(refresh_token) = maybe_auth.refresh_token {
        auth.0.refresh_token = refresh_token;
    }

    Ok(Html(format!(
        r#"<!doctype html>
<html>
  <head><title>NOT LAST.FM: refreshed</title></head>
  <body>
    <h1>ahhhhh</h1>
    <p>refreshing. {}</p>
  </body>
</html>"#,
        CONFIG.get_new_link("back")
    ))
    .into_response())
}

async fn uptime() -> Result<response::Response> {
    let uptime = Utc::now() - *START_TIME;
    let s = uptime.num_seconds() - uptime.num_minutes() * 60;
    let m = uptime.num_minutes() - uptime.num_hours() * 60;
    let h = uptime.num_hours() - uptime.num_days() * 24;
    let d = uptime.num_days();
    Ok(format!("{d}d {h}h {m}m {s}s").into_response())
}

async fn do_db_stuff(session: WritableSession, limit: Option<u32>) -> Result<response::Response> {
    let global_auth = {
        let guard = GLOBAL_AUTH.read().unwrap();
        guard.clone()
    };

    let session_auth = session.get::<SessionAuth>("auth");
    let global_auth_available = global_auth.is_some();
    if let Some(global_auth) = &global_auth {
        write_to_db(global_auth).await?;
    }

    let mut page = String::from(PAGE_HEADER);
    let results = read_from_db(limit).await?;

    if !global_auth_available {
        page.push_str("<p><em>");
        page.push_str(&format!(
            "global auth was not available, this list may not be up to date. please tell zack. {} or {}?",
            CONFIG.authorize_link("authorize"),
            CONFIG.refresh_link("refresh")
        ));
        page.push_str("</em></p>");
    }

    if limit.is_none() {
        page.push_str("<p>");
        page.push_str(&CONFIG.get_new_link("back"));
        page.push_str("</p>");
    }

    if session_auth.is_none() {
        page.push_str(&format!(
            "<p>{} to listen in (make sure your queue is clear, its kinda janky)</p>",
            CONFIG.authorize_link("log in")
        ));
    }

    page.push_str(&make_table(&results, &session_auth));

    if let Some(session_auth) = session_auth.as_ref() {
        page.push_str(
            r#"
<script type=text/javascript>

function addToQueue(id) {
    let req = {
        'mode': 'cors',
        'method': 'PUT',
        'headers': {
            'Authorization': 'Bearer "#,
        );
        page.push_str(&session_auth.0.access_token);
        page.push_str(
            r#"'
        },
        'body': JSON.stringify({
            'uris': [id],
        }),
    };

    fetch('https://api.spotify.com/v1/me/player/play', req)
        .then((response) => console.log(response))
}

for (el of document.getElementsByClassName('add')) {
    const id = el.id;
    el.addEventListener('click', function() {
        console.log('click on ' + id);
        addToQueue(id);
    });
}

</script>
    "#,
        );
    }

    if limit.is_some() {
        page.push_str("<p><em>");
        page.push_str(&CONFIG.show_all_link("show all"));
        page.push_str("</em></p>");
    }

    page.push_str(
        r#"
<script type=text/javascript>
for (el of document.getElementsByClassName('datetime')) {
    let date = new Date(el.innerText);
    if (!isNaN(date.getYear())) {
        el.textContent = date.toLocaleDateString(
            'en-us', {
                year: 'numeric',
                month: 'short',
                day: 'numeric',
                hour: 'numeric',
                minute: 'numeric',
                second: 'numeric',
            }
        );
    }
}
</script>
"#,
    );

    page.push_str(PAGE_FOOTER);

    Ok((
        if global_auth_available {
            StatusCode::OK
        } else {
            StatusCode::ACCEPTED
        },
        TypedHeader(AccessControlAllowOrigin::ANY),
        Html(page),
    )
        .into_response())
}

async fn write_to_db(auth: &GlobalAuth) -> Result<()> {
    let client = request::Client::new();
    let response = client
        .get("https://api.spotify.com/v1/me/player/recently-played")
        .bearer_auth(&auth.0.access_token)
        .query(&[("limit", "50")])
        .send()
        .await
        .map_err(five_hundred!("recently-played request"))?
        .text()
        .await
        .map_err(five_hundred!("recently-played text"))?;

    tracing::debug!("recently-played: {:?}", &response[0..50]);
    let listens: Listens =
        serde_json::from_str(&response).map_err(five_hundred!("recently-played json", response))?;

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect(&CONFIG.db_file)
        .await
        .map_err(five_hundred!("sql pool"))?;

    let mut conn = pool.begin().await.map_err(five_hundred!("start xact"))?;

    for listen in listens.items {
        let mut artist = String::new();
        for (i, a) in listen.track.artists.iter().enumerate() {
            artist.push_str(&a.name);
            if i + 1 != listen.track.artists.len() {
                artist.push_str(", ");
            }
        }

        sqlx::query!(
            "insert or ignore into songs values ($1, $2, $3, $4, $5)",
            listen.track.name,
            listen.track.album.name,
            artist,
            listen.played_at,
            listen.track.id,
        )
        .execute(&mut conn)
        .await
        .map_err(five_hundred!("db insert"))?;
    }

    conn.commit().await.map_err(five_hundred!("xact commit"))?;

    Ok(())
}

async fn read_from_db(limit: Option<u32>) -> Result<Vec<SongRecord>, response::Response> {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect(&CONFIG.db_file)
        .await
        .map_err(five_hundred!("sql pool"))?;

    if let Some(limit) = limit {
        sqlx::query_as!(
            SongRecord,
            "select * from songs order by datetime(date) desc limit $1",
            limit
        )
        .fetch_all(&pool)
        .await
        .map_err(five_hundred!("sql error"))
    } else {
        sqlx::query_as!(
            SongRecord,
            "select * from songs order by datetime(date) desc"
        )
        .fetch_all(&pool)
        .await
        .map_err(five_hundred!("sql error"))
    }
}

// classic function name
fn make_table(results: &[SongRecord], session_auth: &Option<SessionAuth>) -> String {
    let mut table = String::new();

    table.push_str(
        r#"
<table><tr>
<th><b>play</b></th>
<th><b>title</b></th>
<th><b>album</b></th>
<th><b>artists</b></th>
<th><b>time</b></th>
<th><b>id</b></th>
</tr>
"#,
    );

    for result in results {
        table.push_str("<tr>");

        if session_auth.is_some() {
            table.push_str("<td ");
            if let Some(id) = result.id.as_ref() {
                table.push_str(&format!(
                    "style='cursor:pointer;' class='add' id='spotify:track:{}'>▶️</td>",
                    id
                ));
            } else {
                table.push_str("></td>");
            }
        } else {
            table.push_str("<td>");
            table.push_str("</td>");
        }

        table.push_str("<td>");
        if let Some(name) = result.name.as_ref() {
            table.push_str(name);
        }
        table.push_str("</td>");
        table.push_str("<td>");
        if let Some(album) = result.album.as_ref() {
            table.push_str(album);
        }
        table.push_str("</td>");
        table.push_str("<td>");
        if let Some(artist) = result.artist.as_ref() {
            table.push_str(artist);
        }
        table.push_str("</td>");
        table.push_str("<td class='datetime'>");
        if let Some(date) = result.date.as_ref() {
            table.push_str(date);
        }
        table.push_str("</td>");
        table.push_str("<td>");
        if let Some(id) = result.id.as_ref() {
            table.push_str(id);
        }
        table.push_str("</td>");

        table.push_str("</tr>\n");
    }

    table.push_str("</table>");

    table
}

async fn do_oauth2(code: &str, session: &mut WritableSession) -> Result<response::Response> {
    let token_url = Url::parse_with_params(
        spotti::SPOTIFY_TOKEN_URL,
        &[
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("redirect_uri", CONFIG.authorize_url.as_str()),
            ("client_id", &CONFIG.client_id),
            ("client_secret", &CONFIG.client_secret),
        ],
    )
    .map_err(five_hundred!("token_url malformed"))?;
    tracing::trace!("requesting {}", token_url.as_str());

    let client = reqwest::Client::new();
    let response = client
        .post(token_url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Content-Length", "0")
        .send()
        .await
        .map_err(five_hundred!("token request"))?
        .text()
        .await
        .map_err(five_hundred!("token response"))?;

    tracing::debug!("token: {:?}", response);
    let tokens: TokenPair =
        serde_json::from_str(&response).map_err(five_hundred!("token json", response))?;

    let was_me = was_me(&tokens).await?;
    if was_me {
        let mut global_auth = GLOBAL_AUTH
            .write()
            .map_err(five_hundred!("lock for writing (authorize)"))?;
        if let None = *global_auth {
            tracing::info!("deviously stealing credentials");
            *global_auth = Some(GlobalAuth(tokens.clone()));
        }
    }

    tracing::trace!("tokens={:?}", tokens);
    session
        .insert("auth", SessionAuth(tokens))
        .map_err(five_hundred!("token session"))?;

    Ok(Html(format!(
        r#"<!doctype html>
  <head><title>NOT LAST.FM: authorized</title></head>
  <body>
    <h1>nice! you're authorized</h1>
    <p><em>{}</em></p>
    <p>{}</p>
  </body>
</html>"#,
        if was_me {
            "and very handsome at that"
        } else {
            "not globally though :/"
        },
        CONFIG.get_new_link("back"),
    ))
    .into_response())
}

async fn was_me(tokens: &TokenPair) -> Result<bool> {
    let client = request::Client::new();
    let response = client
        .get("https://api.spotify.com/v1/me")
        .bearer_auth(&tokens.access_token)
        .send()
        .await
        .map_err(five_hundred!("get me"))?
        .text()
        .await
        .map_err(five_hundred!("get me response text"))?;

    tracing::debug!("get me: {:?}", response);
    let me = serde_json::from_str::<Me>(&response).map_err(five_hundred!("get me", response))?;
    Ok(me.id == spotti::ME)
}

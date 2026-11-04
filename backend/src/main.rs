use axum::{extract::{Query, State}, http::{header, StatusCode, Uri}, response::IntoResponse, routing::get, Json, Router};
use futures::{stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Build do Angular embutido no binário em release (em debug é lido do disco). Rode `npx ng build` antes do `cargo build`.
#[derive(rust_embed::Embed)]
#[folder = "../frontend/dist/frontend/browser"]
struct Assets;

async fn static_file(uri: Uri) -> impl IntoResponse {
    let path = match uri.path().trim_start_matches('/') {
        "" => "index.html",
        p => p,
    };
    match Assets::get(path) {
        Some(f) => ([(header::CONTENT_TYPE, f.metadata.mimetype().to_string())], f.data).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

#[derive(Serialize, Debug, PartialEq)]
struct Video {
    id: String,
    title: String,
    channel: String,
    published: String,
}

struct AppState {
    channels: Vec<(String, String)>,
    http: reqwest::Client,
}

#[derive(Deserialize)]
struct VideosQuery {
    /// RFC3339 em UTC, ex. 2026-10-28T00:00:00.000Z
    since: String,
}

/// Formato do Takeout: id, url, título. O cabeçalho varia com o idioma, então só aceitamos linhas cujo id começa com UC.
fn parse_subscriptions(csv_text: &str) -> Vec<(String, String)> {
    csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(csv_text.as_bytes())
        .records()
        .filter_map(Result::ok)
        .filter(|r| r.get(0).is_some_and(|id| id.starts_with("UC")))
        .map(|r| (r[0].to_string(), r.get(2).unwrap_or("").to_string()))
        .collect()
}

// shortcut: o RSS do YouTube só traz os 15 uploads mais recentes do canal; para períodos longos em canais muito ativos, migrar pra YouTube Data API.
fn parse_feed(xml: &str) -> Vec<Video> {
    let Ok(doc) = roxmltree::Document::parse(xml) else { return vec![] };
    let child = |n: roxmltree::Node, name: &str| {
        n.children().find(|c| c.tag_name().name() == name).and_then(|c| c.text()).unwrap_or("").to_string()
    };
    doc.root_element()
        .children()
        .filter(|n| n.tag_name().name() == "entry")
        .map(|e| Video {
            id: child(e, "videoId"),
            title: child(e, "title"),
            channel: e.children().find(|c| c.tag_name().name() == "author").map(|a| child(a, "name")).unwrap_or_default(),
            published: child(e, "published"),
        })
        .filter(|v| !v.id.is_empty())
        .collect()
}

async fn fetch_feed(http: &reqwest::Client, channel_id: &str) -> Result<String, reqwest::Error> {
    let url = format!("https://www.youtube.com/feeds/videos.xml?channel_id={channel_id}");
    http.get(url).send().await?.error_for_status()?.text().await
}

async fn videos(State(st): State<Arc<AppState>>, Query(q): Query<VideosQuery>) -> Json<Vec<Video>> {
    let mut all: Vec<Video> = stream::iter(st.channels.clone())
        .map(|(id, title)| {
            let http = st.http.clone();
            async move {
                match fetch_feed(&http, &id).await {
                    Ok(xml) => parse_feed(&xml),
                    Err(e) => {
                        eprintln!("falhou {title} ({id}): {e}");
                        vec![]
                    }
                }
            }
        })
        .buffer_unordered(16)
        .flat_map(stream::iter)
        .collect()
        .await;
    // Datas do feed (2026-11-03T15:00:12+00:00) e o since (…Z) são UTC, então comparar texto equivale a comparar datas.
    all.retain(|v| v.published.as_str() >= q.since.as_str());
    all.sort_by(|a, b| b.published.cmp(&a.published));
    Json(all)
}

#[tokio::main]
async fn main() {
    let csv_path = std::env::args().nth(1).unwrap_or_else(|| "subscriptions.csv".into());
    let text = std::fs::read_to_string(&csv_path).unwrap_or_else(|e| panic!("não consegui ler {csv_path}: {e}"));
    let channels = parse_subscriptions(&text);
    println!("{} canais carregados de {csv_path}", channels.len());

    let http = reqwest::Client::builder().timeout(std::time::Duration::from_secs(15)).build().unwrap();
    let app = Router::new()
        .route("/api/videos", get(videos))
        .fallback(static_file)
        .with_state(Arc::new(AppState { channels, http }));

    // 0.0.0.0 = escuta na rede local, pra abrir no celular
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("abrindo em http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_do_takeout() {
        let csv = "ID do canal,URL do canal,Título do canal\n\
            UC1,http://www.youtube.com/channel/UC1,Felps\n\
            UC2,http://www.youtube.com/channel/UC2,\"Rena Malik, M.D.\"\n\n\n";
        assert_eq!(
            parse_subscriptions(csv),
            vec![("UC1".into(), "Felps".into()), ("UC2".into(), "Rena Malik, M.D.".into())]
        );
    }

    #[test]
    fn feed_rss() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns:yt="http://www.youtube.com/xml/schemas/2015" xmlns="http://www.w3.org/2005/Atom">
 <title>Canal</title>
 <entry>
  <id>yt:video:abc123</id>
  <yt:videoId>abc123</yt:videoId>
  <title>Vídeo &amp; teste</title>
  <author><name>Canal X</name></author>
  <published>2026-11-03T15:00:12+00:00</published>
 </entry>
</feed>"#;
        assert_eq!(
            parse_feed(xml),
            vec![Video { id: "abc123".into(), title: "Vídeo & teste".into(), channel: "Canal X".into(), published: "2026-11-03T15:00:12+00:00".into() }]
        );
    }
}

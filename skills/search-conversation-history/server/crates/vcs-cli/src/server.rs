//! Local HTTP bridge.
//!
//! The desktop shell calls the service through Tauri commands. During interface development the same
//! service is reachable over loopback so a browser can drive the real engine instead of a mock. The
//! listener binds 127.0.0.1 only.

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tiny_http::{Header, Request, Response, Server};
use vcs_service::{bench, integrations, map, App, IndexRequest};

fn json_response(body: String, status: u16) -> Response<std::io::Cursor<Vec<u8>>> {
    let header = Header::from_bytes(&b"Content-Type"[..], &b"application/json; charset=utf-8"[..])
        .expect("static header");
    Response::from_string(body).with_status_code(status).with_header(header)
}

fn ok(value: &impl serde::Serialize) -> Response<std::io::Cursor<Vec<u8>>> {
    match serde_json::to_string(value) {
        Ok(body) => json_response(body, 200),
        Err(error) => json_response(
            serde_json::json!({ "error": error.to_string() }).to_string(),
            500,
        ),
    }
}

fn failed(error: &str, status: u16) -> Response<std::io::Cursor<Vec<u8>>> {
    json_response(serde_json::json!({ "error": error }).to_string(), status)
}

fn query_parameters(url: &str) -> HashMap<String, String> {
    let mut parameters = HashMap::new();
    if let Some((_, query)) = url.split_once('?') {
        for pair in query.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                parameters.insert(key.to_string(), decode(value));
            }
        }
    }
    parameters
}

fn decode(value: &str) -> String {
    let bytes = value.replace('+', " ").into_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(&String::from_utf8_lossy(&bytes[index + 1..index + 3]), 16) {
                out.push(byte);
                index += 3;
                continue;
            }
        }
        out.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn read_body(request: &mut Request) -> Result<serde_json::Value> {
    let mut body = String::new();
    std::io::Read::read_to_string(request.as_reader(), &mut body)?;
    if body.trim().is_empty() {
        return Ok(serde_json::json!({}));
    }
    Ok(serde_json::from_str(&body)?)
}

fn content_type(path: &std::path::Path) -> &'static str {
    match path.extension().and_then(|value| value.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("json") => "application/json; charset=utf-8",
        Some("png") => "image/png",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    }
}

/// Serve one built interface file, refusing any path that escapes the directory.
fn static_file(root: &std::path::Path, url: &str) -> Option<Response<std::fs::File>> {
    let path = url.split('?').next().unwrap_or("/");
    let relative = path.trim_start_matches('/');
    if relative.split('/').any(|part| part == "..") {
        return None;
    }
    let mut candidate = root.join(relative);
    if candidate.is_dir() || relative.is_empty() {
        candidate = root.join("index.html");
    }
    if !candidate.starts_with(root) || !candidate.is_file() {
        candidate = root.join("index.html");
    }
    let file = std::fs::File::open(&candidate).ok()?;
    let header = Header::from_bytes(&b"Content-Type"[..], content_type(&candidate).as_bytes()).ok()?;
    Some(Response::from_file(file).with_header(header))
}

pub fn serve(app: App, address: &str, ui: Option<PathBuf>) -> Result<()> {
    if !address.starts_with("127.0.0.1:") && !address.starts_with("localhost:") {
        return Err(anyhow!(
            "the bridge only binds loopback; {address} would expose the index beyond this computer"
        ));
    }
    let server = Server::http(address).map_err(|error| anyhow!("cannot bind {address}: {error}"))?;
    eprintln!("fast-conversation-search bridge on http://{address}");
    eprintln!("index: {}", app.database().display());
    let app = Arc::new(app);
    let server = Arc::new(server);
    let ui = Arc::new(ui);

    // The speed comparison starts both searches at the same moment, so the bridge must answer more
    // than one request at a time. Four workers is enough for one interface.
    let mut workers = Vec::new();
    for _ in 0..4 {
        let server = Arc::clone(&server);
        let app = Arc::clone(&app);
        let ui = Arc::clone(&ui);
        workers.push(std::thread::spawn(move || worker(server, app, ui)));
    }
    for worker in workers {
        let _ = worker.join();
    }
    Ok(())
}

fn worker(server: Arc<Server>, app: Arc<App>, ui: Arc<Option<PathBuf>>) {
    for mut request in server.incoming_requests() {
        let url = request.url().to_string();
        let route = url.split('?').next().unwrap_or("").to_string();
        let parameters = query_parameters(&url);
        let app = Arc::clone(&app);

        let response = match (request.method().as_str(), route.as_str()) {
            ("GET", "/api/probe") => ok(&app.probe()),
            ("GET", "/api/detect") => {
                let days = parameters.get("days").and_then(|value| value.parse().ok());
                ok(&app.detect(days))
            }
            ("POST", "/api/index/start") => match read_body(&mut request) {
                Ok(body) => {
                    let request: IndexRequest = serde_json::from_value(body).unwrap_or(IndexRequest {
                        sources: Vec::new(),
                        days: None,
                    });
                    let background = Arc::clone(&app);
                    std::thread::spawn(move || {
                        let _ = background.build_index(&request);
                    });
                    ok(&serde_json::json!({ "started": true }))
                }
                Err(error) => failed(&error.to_string(), 400),
            },
            ("POST", "/api/index/cancel") => {
                app.cancel();
                ok(&serde_json::json!({ "cancelled": true }))
            }
            ("GET", "/api/index/status") => ok(&app.progress()),
            ("GET", "/api/totals") => match app.totals() {
                Ok((total, scopes)) => ok(&serde_json::json!({ "total": total, "scopes": scopes })),
                Err(error) => failed(&error.to_string(), 500),
            },
            ("GET", "/api/suggest") => {
                let count = parameters.get("count").and_then(|v| v.parse().ok()).unwrap_or(3);
                match app.suggested_phrases(count) {
                    Ok(phrases) => ok(&phrases),
                    Err(error) => failed(&error.to_string(), 500),
                }
            }
            ("GET", "/api/search") => {
                let query = parameters.get("q").cloned().unwrap_or_default();
                let limit = parameters.get("limit").and_then(|v| v.parse().ok()).unwrap_or(50);
                let order = vcs_core::index::Order::from(parameters.get("order").map(String::as_str));
                match app.search_ordered(&query, limit, order) {
                    Ok(response) => ok(&response),
                    Err(error) => failed(&error.to_string(), 500),
                }
            }
            ("GET", "/api/context") => {
                let session = parameters.get("session").cloned().unwrap_or_default();
                let timestamp = parameters.get("timestamp").cloned();
                let span = parameters.get("span").and_then(|v| v.parse().ok()).unwrap_or(3);
                match app.context(&session, timestamp.as_deref(), span) {
                    Ok(hits) => ok(&hits),
                    Err(error) => failed(&error.to_string(), 500),
                }
            }
            ("GET", "/api/raw-search") => {
                let query = parameters.get("q").cloned().unwrap_or_default();
                ok(&bench::raw_only(&query))
            }
            ("GET", "/api/raw-samples") => {
                let query = parameters.get("q").cloned().unwrap_or_default();
                let limit = parameters.get("limit").and_then(|v| v.parse().ok()).unwrap_or(5);
                ok(&bench::raw_samples(&query, limit))
            }
            ("GET", "/api/bench") => {
                let query = parameters.get("q").cloned().unwrap_or_default();
                let limit = parameters.get("limit").and_then(|v| v.parse().ok()).unwrap_or(200);
                match bench::run(&app, &query, limit) {
                    Ok(result) => ok(&result),
                    Err(error) => failed(&error.to_string(), 500),
                }
            }
            ("GET", "/api/map") => {
                let days = parameters.get("days").and_then(|v| v.parse().ok()).unwrap_or(7);
                match map::build(&app, days) {
                    Ok(result) => ok(&result),
                    Err(error) => failed(&error.to_string(), 500),
                }
            }
            ("GET", "/api/integrations") => {
                let executable = std::env::current_exe().unwrap_or_default();
                ok(&integrations::plan(
                    &vcs_adapters::home_directory(),
                    &executable,
                    app.database(),
                ))
            }
            ("POST", "/api/integrations/apply") => match read_body(&mut request) {
                Ok(body) => {
                    let tools: Vec<String> = body
                        .get("tools")
                        .and_then(|value| value.as_array())
                        .map(|values| {
                            values.iter().filter_map(|v| v.as_str()).map(str::to_string).collect()
                        })
                        .unwrap_or_default();
                    let confirmed = body.get("confirmed").and_then(|v| v.as_bool()).unwrap_or(false);
                    let executable = std::env::current_exe().unwrap_or_default();
                    match integrations::apply(
                        &vcs_adapters::home_directory(),
                        &executable,
                        app.database(),
                        &tools,
                        confirmed,
                    ) {
                        Ok(outcomes) => ok(&outcomes),
                        Err(error) => failed(&error.to_string(), 400),
                    }
                }
                Err(error) => failed(&error.to_string(), 400),
            },
            ("POST", "/api/integrations/remove") => match read_body(&mut request) {
                Ok(body) => {
                    let tools: Vec<String> = body
                        .get("tools")
                        .and_then(|value| value.as_array())
                        .map(|values| {
                            values.iter().filter_map(|v| v.as_str()).map(str::to_string).collect()
                        })
                        .unwrap_or_default();
                    match integrations::remove(&vcs_adapters::home_directory(), &tools) {
                        Ok(outcomes) => ok(&outcomes),
                        Err(error) => failed(&error.to_string(), 400),
                    }
                }
                Err(error) => failed(&error.to_string(), 400),
            },
            ("GET", _) => {
                if let Some(root) = ui.as_ref().as_ref() {
                    if let Some(response) = static_file(root, &url) {
                        let _ = request.respond(response);
                        continue;
                    }
                }
                failed("not found", 404)
            }
            _ => failed("not found", 404),
        };
        let _ = request.respond(response);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bridge_refuses_to_bind_beyond_loopback() {
        let app = App::new(std::env::temp_dir().join("vcs-bind-test.sqlite"));
        let error = serve(app, "0.0.0.0:8787", None).unwrap_err();
        assert!(error.to_string().contains("only binds loopback"));
    }

    #[test]
    fn query_parameters_are_percent_decoded() {
        let parameters = query_parameters("/api/search?q=trigram%20index&limit=10");
        assert_eq!(parameters.get("q").unwrap(), "trigram index");
        assert_eq!(parameters.get("limit").unwrap(), "10");
    }

    #[test]
    fn a_traversing_static_path_is_refused() {
        let root = std::env::temp_dir().join(format!("vcs-static-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("index.html"), "<html></html>").unwrap();
        assert!(static_file(&root, "/../../etc/passwd").is_none());
        assert!(static_file(&root, "/").is_some());
        std::fs::remove_dir_all(&root).unwrap();
    }
}

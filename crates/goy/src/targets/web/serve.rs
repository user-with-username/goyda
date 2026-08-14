use anyhow::{Context, Result};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::term;

const PREFERRED_PORT: u16 = 4173;

/// Bumped every time [`serve_and_open`] is called after the first (i.e.
/// every `r`/`R` reload, once the wasm has already been rebuilt onto
/// disk) - polled by the live-reload script `templates/index.html`
/// injects, via [`handle_connection`]'s `/__goyda_reload` route.
static RELOAD_GENERATION: AtomicU64 = AtomicU64::new(0);

/// The dev server binds its port exactly once for the process's lifetime -
/// there's nothing to rebind on a reload (the wasm's already been rebuilt
/// onto disk by the time this runs again; every request already reads
/// straight from disk with no caching - see [`handle_connection`]), so a
/// second/third/... call just bumps [`RELOAD_GENERATION`] and returns
/// immediately instead of trying (and failing) to bind the same port again.
static SERVER_STARTED: OnceLock<()> = OnceLock::new();

pub fn serve_and_open(dist_dir: &Path) -> Result<()> {
    if SERVER_STARTED.get().is_some() {
        RELOAD_GENERATION.fetch_add(1, Ordering::Relaxed);
        term::info("rebuilt - the dev server will pick it up on the next reload/poll");
        return Ok(());
    }

    let s = term::spinner_step("starting web server");
    let listener = bind_server(PREFERRED_PORT);
    let listener = match listener {
        Ok(listener) => {
            s.ok();
            listener
        }
        Err(e) => {
            s.fail(&e.to_string());
            return Err(e);
        }
    };
    let port = listener
        .local_addr()
        .context("Failed to read local dev server address")?
        .port();
    let url = format!("http://127.0.0.1:{port}/");

    term::info(&format!("{url} (Ctrl+C to stop)"));
    open_browser(&url);

    let _ = SERVER_STARTED.set(());

    let dist_dir = dist_dir.to_path_buf();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    if let Err(e) = handle_connection(stream, &dist_dir) {
                        eprintln!("goyda(web): request error: {e}");
                    }
                }
                Err(e) => eprintln!("goyda(web): connection error: {e}"),
            }
        }
    });

    Ok(())
}

fn bind_server(preferred_port: u16) -> Result<TcpListener> {
    if let Ok(listener) = TcpListener::bind(("127.0.0.1", preferred_port)) {
        return Ok(listener);
    }
    TcpListener::bind(("127.0.0.1", 0)).context("Failed to bind the local dev server to any port")
}

fn open_browser(url: &str) {
    let result = if cfg!(target_os = "windows") {
        Command::new("cmd").args(&["/C", "start", "", url]).status()
    } else if cfg!(target_os = "macos") {
        Command::new("open").arg(url).status()
    } else {
        Command::new("xdg-open").arg(url).status()
    };

    if result.map(|s| !s.success()).unwrap_or(true) {
        term::info(&format!(
            "Could not open a browser automatically - open {url} manually."
        ));
    }
}

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js" | "mjs") => "text/javascript; charset=utf-8",
        Some("wasm") => "application/wasm",
        Some("css") => "text/css; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("ico") => "image/x-icon",
        _ => "application/octet-stream",
    }
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    out.push(byte);
                    i += 3;
                    continue;
                }
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn resolve_requested_file(dist_dir: &Path, url_path: &str) -> PathBuf {
    let relative = if url_path == "/" {
        "index.html"
    } else {
        url_path.trim_start_matches('/')
    };
    let decoded = percent_decode(relative);

    // Reject any path that escapes the dist directory (e.g. via `..`).
    let candidate = dist_dir.join(&decoded);
    let is_safe = decoded.split(['/', '\\']).all(|part| part != "..");

    if is_safe {
        candidate
    } else {
        dist_dir.join("index.html")
    }
}

fn handle_connection(stream: TcpStream, dist_dir: &Path) -> Result<()> {
    let mut reader = BufReader::new(
        stream
            .try_clone()
            .context("Failed to clone connection stream")?,
    );
    let mut stream = stream;

    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;

    loop {
        let mut header_line = String::new();
        let read = reader.read_line(&mut header_line)?;
        if read == 0 || header_line == "\r\n" {
            break;
        }
    }

    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let raw_path = parts.next().unwrap_or("/");

    if method != "GET" && method != "HEAD" {
        return write_response(
            &mut stream,
            405,
            "Method Not Allowed",
            "text/plain",
            b"Method Not Allowed",
        );
    }

    let url_path = raw_path.split('?').next().unwrap_or("/");

    if url_path == "/__goyda_reload" {
        let id = RELOAD_GENERATION.load(Ordering::Relaxed).to_string();
        return write_response(
            &mut stream,
            200,
            "OK",
            "text/plain; charset=utf-8",
            id.as_bytes(),
        );
    }

    let file_path = resolve_requested_file(dist_dir, url_path);

    match fs::read(&file_path) {
        Ok(body) => write_response(&mut stream, 200, "OK", content_type(&file_path), &body),
        // SPA fallback: an extensionless path that isn't a real file is a
        // client-side route (e.g. `/about`), not a missing asset - serve
        // `index.html` so goyda can read the URL and mount the right
        // `#[page(...)]` itself, same as a direct link or a page refresh.
        Err(_) if file_path.extension().is_none() => match fs::read(dist_dir.join("index.html")) {
            Ok(body) => write_response(&mut stream, 200, "OK", "text/html; charset=utf-8", &body),
            Err(_) => write_response(
                &mut stream,
                404,
                "Not Found",
                "text/plain",
                b"404 Not Found",
            ),
        },
        Err(_) => write_response(
            &mut stream,
            404,
            "Not Found",
            "text/plain",
            b"404 Not Found",
        ),
    }
}

fn write_response(
    stream: &mut TcpStream,
    code: u16,
    reason: &str,
    content_type: &str,
    body: &[u8],
) -> Result<()> {
    let header = format!(
        "HTTP/1.1 {code} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()?;
    Ok(())
}

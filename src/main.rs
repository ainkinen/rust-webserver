use std::path::Path;
use std::time::Duration;
use std::{env, fs, io};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

const HTML_ROOT: &str = "html_root";

#[tokio::main]
async fn main() -> io::Result<()> {
    let port = env::var("PORT").unwrap_or("8080".to_string());
    // TODO: Bind without string formatting
    let listener = TcpListener::bind(format!("127.0.0.1:{port}")).await?;

    let local_addr = listener.local_addr()?;
    println!("Listening on: {local_addr}");

    loop {
        let (stream, peer_addr) = listener.accept().await?;
        tokio::spawn(async move {
            println!("Connection from {peer_addr} established!");
            let _ = handle_connection(stream).await;
        });
    }
}

async fn handle_connection(mut stream: TcpStream) -> io::Result<()> {
    let buf_reader = BufReader::new(&mut stream);

    let line = buf_reader.lines().next_line().await;

    let request_line = line?.unwrap();

    let request_line_parts: Vec<_> = request_line.split(" ").collect();
    let [_method, request_path, _http_version] = request_line_parts.try_into().unwrap();

    let request_path = match request_path {
        "/" => "hello.html", // Use hello as the default index page
        "/sleep" => {
            tokio::time::sleep(Duration::from_secs(3)).await;
            "hello.html"
        }
        path if path.starts_with("/") => &path[1..], // Strip leading /
        _ => panic!("Absolute request path not supported"),
    };

    // Canonicalize the path and check that we are still inside the html root
    let html_root = Path::new(HTML_ROOT).canonicalize()?;
    let file_path = Path::join(&html_root, request_path);

    let (status_line, content_path) = if exists_within(&html_root, &file_path) {
        ("HTTP/1.1 200 OK", file_path)
    } else {
        ("HTTP/1.1 404 NOT FOUND", Path::join(&html_root, "404.html"))
    };

    let contents = fs::read_to_string(content_path)?;
    let length = contents.len();

    let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");

    stream.write_all(response.as_bytes()).await
}

fn exists_within(canonical_root: &Path, path: &Path) -> bool {
    if let Ok(canonical_path) = path.canonicalize() {
        return canonical_path.starts_with(canonical_root);
    }

    false
}

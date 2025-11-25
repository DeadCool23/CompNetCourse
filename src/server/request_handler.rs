use log::{info, warn, error, debug};
use std::fs;
use std::io::{Read, Write, BufReader, BufWriter};
use std::net::TcpStream;
use std::path::Path;

use super::http_status::HttpStatus;

static MIME_TYPES: &[(&str, &str)] = &[
    ("html", "text/html"),
    ("css", "text/css"),
    ("js", "application/javascript"),
    ("png", "image/png"),
    ("jpg", "image/jpeg"),
    ("jpeg", "image/jpeg"),
    ("gif", "image/gif"),
    ("svg", "image/svg+xml"),
    ("ico", "image/x-icon"),
    ("json", "application/json"),
    ("txt", "text/plain"),
];

pub fn handle_client(mut stream: TcpStream, document_root: &Path, max_file_size: u64) {
    let peer_addr = match stream.peer_addr() {
        Ok(addr) => addr.to_string(),
        Err(_) => "unknown".to_string(),
    };

    debug!("Handling request from {}", peer_addr);

    let mut buffer = [0u8; 8192];
    let bytes_read = match stream.read(&mut buffer) {
        Ok(0) => {
            debug!("Connection closed by client {}", peer_addr);
            return;
        }
        Ok(n) => n,
        Err(e) => {
            error!("Error reading from {}: {}", peer_addr, e);
            return;
        }
    };

    let request = String::from_utf8_lossy(&buffer[..bytes_read]);
    let request_lines: Vec<&str> = request.lines().collect();

    if request_lines.is_empty() {
        send_response(&mut stream, HttpStatus::BadRequest, "");
        return;
    }

    let first_line: Vec<&str> = request_lines[0].split_whitespace().collect();
    if first_line.len() < 2 {
        send_response(&mut stream, HttpStatus::BadRequest, "");
        return;
    }

    let method = first_line[0];
    let mut path = first_line[1];

    if path.contains("..") {
        warn!("Path traversal attempt from {}: {}", peer_addr, path);
        send_response(&mut stream, HttpStatus::Forbidden, "");
        return;
    }

    if path == "/" {
        path = "/index.html";
    }

    let file_path = document_root.join(&path[1..]);

    match method {
        "GET" | "HEAD" => handle_file_request(&mut stream, &file_path, method == "HEAD", &peer_addr, max_file_size),
        _ => {
            warn!("Unsupported method from {}: {}", peer_addr, method);
            send_response(&mut stream, HttpStatus::MethodNotAllowed, "");
        }
    }
}

fn handle_file_request(
    stream: &mut TcpStream, 
    file_path: &Path, 
    is_head: bool, 
    client_addr: &str,
    max_file_size: u64
) {
    if !file_path.exists() {
        info!("File not found for {}: {:?}", client_addr, file_path);
        send_response(stream, HttpStatus::NotFound, "");
        return;
    }

    if !file_path.is_file() {
        warn!("Attempt to access directory from {}: {:?}", client_addr, file_path);
        send_response(stream, HttpStatus::Forbidden, "");
        return;
    }

    let metadata = match std::fs::metadata(file_path) {
        Ok(meta) => meta,
        Err(e) => {
            error!("Error getting metadata for {:?}: {}", file_path, e);
            send_response(stream, HttpStatus::InternalServerError, "");
            return;
        }
    };

    if metadata.len() > max_file_size {
        warn!("File too large for {}: {:?} ({} bytes)", 
              client_addr, file_path, metadata.len());
        send_response(stream, HttpStatus::PayloadTooLarge, "");
        return;
    }

    let ext = file_path.extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    
    let content_type = MIME_TYPES.iter()
        .find(|(e, _)| *e == ext)
        .map(|(_, mime)| *mime)
        .unwrap_or("application/octet-stream");

    if is_head {
        let headers = format!(
            "{}Content-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            HttpStatus::Ok.as_response_line(),
            content_type,
            metadata.len()
        );
        if let Err(e) = stream.write_all(headers.as_bytes()) {
            error!("Error sending HEAD response to {}: {}", client_addr, e);
        }
    } else {
        match fs::File::open(file_path) {
            Ok(file) => {
                let mut reader = BufReader::new(file);
                let mut writer = BufWriter::new(stream);
                
                let headers = format!(
                    "{}Content-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    HttpStatus::Ok.as_response_line(),
                    content_type,
                    metadata.len()
                );

                if let Err(e) = writer.write_all(headers.as_bytes()) {
                    error!("Error sending headers to {}: {}", client_addr, e);
                    return;
                }

                let mut buffer = [0u8; 8192];
                loop {
                    match reader.read(&mut buffer) {
                        Ok(0) => break,
                        Ok(n) => {
                            if let Err(e) = writer.write_all(&buffer[..n]) {
                                error!("Error sending file data to {}: {}", client_addr, e);
                                break;
                            }
                        }
                        Err(e) => {
                            error!("Error reading file for {}: {}", client_addr, e);
                            break;
                        }
                    }
                }

                if let Err(e) = writer.flush() {
                    error!("Error flushing stream for {}: {}", client_addr, e);
                }
            }
            Err(e) => {
                error!("Error opening file {:?} for {}: {}", file_path, client_addr, e);
                send_response(stream, HttpStatus::InternalServerError, "");
            }
        }
    }

    info!("Served file to {}: {:?} ({} bytes)", client_addr, file_path, metadata.len());
}

fn send_response(stream: &mut TcpStream, status: HttpStatus, body: &str) {
    let response = format!(
        "{}Content-Length: {}\r\nConnection: close\r\n\r\n{}",
        status.as_response_line(),
        body.len(),
        body
    );

    if let Err(e) = stream.write_all(response.as_bytes()) {
        error!("Error sending response: {}", e);
    }
}
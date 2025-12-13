use std::sync::Arc;
use std::io::{Read, Seek, Write};
use log::{debug, error, info, warn};

use super::http_status::HttpStatus;
use super::connection::ConnectionStage;
use super::connection_manager::ConnectionManager;

pub fn handle_readable_in_pool(
    fd: i32,
    connection_manager: Arc<ConnectionManager>,
    doc_root: std::path::PathBuf,
    max_file_size: u64,
) {
    debug!(
        "[Thread {:?}] Handling readable connection fd {}",
        std::thread::current().id(),
        fd
    );

    connection_manager.with_connection(fd, |conn| {
        if conn.stage != ConnectionStage::Recv {
            return;
        }

        let bytes_read = match conn.stream.read(&mut conn.request_buffer[conn.request_len..]) {
            Ok(0) => {
                debug!("Connection closed by client on fd {}", fd);
                conn.stage = ConnectionStage::Close;
                return;
            }
            Ok(n) => {
                debug!("Read {} bytes from fd {}", n, fd);
                n
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                return;
            }
            Err(e) => {
                error!("Error reading from connection {}: {}", fd, e);
                conn.stage = ConnectionStage::Close;
                return;
            }
        };

        conn.request_len += bytes_read;

        let buffer_slice = &conn.request_buffer[..conn.request_len];
        if contains_double_newline(buffer_slice) {
            debug!(
                "Full request received on fd {} ({} bytes)",
                fd, conn.request_len
            );

            let request_data = buffer_slice[..conn.request_len].to_vec();
            let request_str = String::from_utf8_lossy(&request_data);

            conn.request_len = 0;
            conn.stage = ConnectionStage::Parse;

            match parse_http_request(
                &request_str,
                &doc_root,
                max_file_size,
                fd,
            ) {
                Ok((headers, file, file_size, is_head)) => {
                    conn.headers = headers;
                    conn.headers_sent = 0;
                    conn.file = file;
                    conn.file_size = file_size;
                    conn.is_head = is_head;
                    conn.stage = ConnectionStage::SendHeaders;
                    
                    debug!("Request parsed and ready to send headers on fd {}", fd);
                }
                Err(error_headers) => {
                    conn.headers = error_headers;
                    conn.headers_sent = 0;
                    conn.stage = ConnectionStage::SendHeaders;
                    debug!("Error response ready to send on fd {}", fd);
                }
            }
        }
    });
}

pub fn handle_writable_in_pool(fd: i32, connection_manager: Arc<ConnectionManager>) {
    debug!(
        "[Thread {:?}] Handling writable connection fd {}",
        std::thread::current().id(),
        fd
    );

    connection_manager.with_connection(fd, |conn| {
        match conn.stage {
            ConnectionStage::SendHeaders => {
                if conn.headers_sent < conn.headers.len() {
                    match conn.stream.write(&conn.headers[conn.headers_sent..]) {
                        Ok(0) => {
                            debug!("Connection closed while sending headers on fd {}", fd);
                            conn.stage = ConnectionStage::Close;
                            return;
                        }
                        Ok(n) => {
                            debug!("Sent {} header bytes on fd {}", n, fd);
                            conn.headers_sent += n;
                            if conn.headers_sent >= conn.headers.len() {
                                if conn.is_head || conn.file.is_none() {
                                    info!("Headers sent for HEAD request on fd {}", fd);
                                    conn.stage = ConnectionStage::Close;
                                } else {
                                    debug!("Headers sent, starting file transfer on fd {}", fd);
                                    conn.stage = ConnectionStage::SendFile;
                                }
                            }
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            return;
                        }
                        Err(e) => {
                            error!("Error writing headers to fd {}: {}", fd, e);
                            conn.stage = ConnectionStage::Close;
                        }
                    }
                }
            }

            ConnectionStage::SendFile => {
                if let Some(ref mut file) = conn.file {
                    let mut buffer = [0u8; 65536];
                    match file.read(&mut buffer) {
                        Ok(0) => {
                            info!(
                                "File sent completely on fd {} ({} bytes)",
                                fd, conn.file_sent
                            );
                            conn.stage = ConnectionStage::Close;
                        }
                        Ok(bytes_read) => match conn.stream.write(&buffer[..bytes_read]) {
                            Ok(0) => {
                                debug!("Connection closed while sending file on fd {}", fd);
                                conn.stage = ConnectionStage::Close;
                            }
                            Ok(bytes_written) => {
                                conn.file_sent += bytes_written as u64;
                                debug!(
                                    "Sent {} file bytes on fd {} (total: {}/{})",
                                    bytes_written, fd, conn.file_sent, conn.file_size
                                );

                                if conn.file_sent >= conn.file_size {
                                    info!(
                                        "File sent completely on fd {} ({} bytes)",
                                        fd, conn.file_sent
                                    );
                                    conn.stage = ConnectionStage::Close;
                                } else if bytes_written < bytes_read {
                                    file.seek(std::io::SeekFrom::Current(
                                        -(bytes_read as i64 - bytes_written as i64),
                                    ))
                                    .ok();
                                }
                            }
                            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                file.seek(std::io::SeekFrom::Current(-(bytes_read as i64)))
                                    .ok();
                            }
                            Err(e) => {
                                error!("Error writing file to fd {}: {}", fd, e);
                                conn.stage = ConnectionStage::Close;
                            }
                        },
                        Err(e) => {
                            error!("Error reading file on fd {}: {}", fd, e);
                            conn.stage = ConnectionStage::Close;
                        }
                    }
                } else {
                    warn!("No file to send on fd {}", fd);
                    conn.stage = ConnectionStage::Close;
                }
            }

            _ => {}
        }
    });
}


fn parse_http_request(
    request_str: &str,
    doc_root: &std::path::PathBuf,
    max_file_size: u64,
    fd: i32,
) -> Result<(Vec<u8>, Option<std::fs::File>, u64, bool), Vec<u8>> {
    let request_lines: Vec<&str> = request_str.lines().collect();

    if request_lines.is_empty() {
        return Err(format_error_response(HttpStatus::BadRequest));
    }

    let first_line: Vec<&str> = request_lines[0].split_whitespace().collect();
    if first_line.len() < 2 {
        return Err(format_error_response(HttpStatus::BadRequest));
    }

    let method = first_line[0];
    let mut path = first_line[1];

    debug!("Parsing request: {} {}", method, path);

    if path.contains("..") {
        warn!("Path traversal attempt on fd {}: {}", fd, path);
        return Err(format_error_response(HttpStatus::Forbidden));
    }

    if path == "/" {
        path = "/index.html";
    }

    let file_path = doc_root.join(&path[1..]);

    if !file_path.exists() {
        info!("File not found: {:?}", file_path);
        return Err(format_error_response(HttpStatus::NotFound));
    }

    if !file_path.is_file() {
        warn!("Attempt to access directory: {:?}", file_path);
        return Err(format_error_response(HttpStatus::Forbidden));
    }

    let metadata = match std::fs::metadata(&file_path) {
        Ok(meta) => meta,
        Err(e) => {
            error!("Error getting metadata for {:?}: {}", file_path, e);
            return Err(format_error_response(HttpStatus::InternalServerError));
        }
    };

    let file_size = metadata.len();
    if file_size > max_file_size {
        warn!("File too large: {:?} ({} > {})", file_path, file_size, max_file_size);
        return Err(format_error_response(HttpStatus::PayloadTooLarge));
    }

    let content_type = get_content_type(&file_path);
    let is_head = method == "HEAD";

    let file = if !is_head {
        match std::fs::File::open(&file_path) {
            Ok(file) => {
                debug!("File opened for fd {}: {} bytes", fd, file_size);
                Some(file)
            }
            Err(e) => {
                error!("Error opening file {:?}: {}", file_path, e);
                return Err(format_error_response(HttpStatus::InternalServerError));
            }
        }
    } else {
        debug!("HEAD request for {:?}", file_path);
        None
    };

    let headers = format!(
        "{}Content-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        HttpStatus::Ok.as_response_line(),
        content_type,
        file_size
    );

    Ok((headers.into_bytes(), file, file_size, is_head))
}



fn format_error_response(status: HttpStatus) -> Vec<u8> {
    let body = match status {
        HttpStatus::NotFound => "<html><body><h1>404 Not Found</h1></body></html>",
        HttpStatus::Forbidden => "<html><body><h1>403 Forbidden</h1></body></html>",
        HttpStatus::BadRequest => "<html><body><h1>400 Bad Request</h1></body></html>",
        HttpStatus::PayloadTooLarge => {
            "<html><body><h1>413 Payload Too Large</h1></body></html>"
        }
        HttpStatus::InternalServerError => {
            "<html><body><h1>500 Internal Server Error</h1></body></html>"
        }
        _ => "<html><body><h1>Error</h1></body></html>",
    };

    format!(
        "{}Content-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status.as_response_line(),
        body.len(),
        body
    )
    .into_bytes()
}

fn get_content_type(file_path: &std::path::PathBuf) -> &'static str {
    let ext = file_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    let mime_types = &[
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

    mime_types
        .iter()
        .find(|(e, _)| *e == ext)
        .map(|(_, mime)| *mime)
        .unwrap_or("application/octet-stream")
}

fn contains_double_newline(buffer: &[u8]) -> bool {
    let len = buffer.len();
    for i in 0..len.saturating_sub(3) {
        if buffer[i] == b'\r'
            && buffer[i + 1] == b'\n'
            && buffer[i + 2] == b'\r'
            && buffer[i + 3] == b'\n'
        {
            return true;
        }
    }

    for i in 0..len.saturating_sub(1) {
        if buffer[i] == b'\n' && buffer[i + 1] == b'\n' {
            return true;
        }
    }

    false
}
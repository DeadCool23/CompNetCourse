use log::debug;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc;

use super::connection_manager::ConnectionManager;
use super::http_status::HttpStatus;

#[derive(Clone)]
pub struct RequestParser {
    doc_root: PathBuf,
    max_file_size: u64,
}

impl RequestParser {
    pub fn new(doc_root: PathBuf, max_file_size: u64) -> Self {
        Self {
            doc_root,
            max_file_size,
        }
    }

    pub fn parse_request_in_thread(
        &self,
        fd: i32,
        request_data: Vec<u8>,
        parse_tx: mpsc::Sender<(i32, Vec<u8>)>,
        conn_manager: Arc<ConnectionManager>,
    ) {
        let request_str = String::from_utf8_lossy(&request_data);
        let request_lines: Vec<&str> = request_str.lines().collect();

        if request_lines.is_empty() {
            debug!("Empty request on fd {}", fd);
            Self::send_error_response(fd, HttpStatus::BadRequest, &parse_tx);
            return;
        }

        let first_line: Vec<&str> = request_lines[0].split_whitespace().collect();
        if first_line.len() < 2 {
            debug!("Malformed request line on fd {}: {}", fd, request_lines[0]);
            Self::send_error_response(fd, HttpStatus::BadRequest, &parse_tx);
            return;
        }

        let method = first_line[0];
        let mut path = first_line[1];
        debug!("Request on fd {}: {} {}", fd, method, path);

        if path.contains("..") {
            log::warn!("Path traversal attempt on fd {}: {}", fd, path);
            Self::send_error_response(fd, HttpStatus::Forbidden, &parse_tx);
            return;
        }

        if path == "/" {
            path = "/index.html";
        }

        let file_path = self.doc_root.join(&path[1..]);

        if !self.validate_and_prepare_response(fd, &file_path, method, parse_tx, conn_manager) {
            return;
        }
    }

    fn validate_and_prepare_response(
        &self,
        fd: i32,
        file_path: &PathBuf,
        method: &str,
        parse_tx: mpsc::Sender<(i32, Vec<u8>)>,
        conn_manager: Arc<ConnectionManager>,
    ) -> bool {
        if !file_path.exists() {
            log::info!("File not found for fd {}: {:?}", fd, file_path);
            Self::send_error_response(fd, HttpStatus::NotFound, &parse_tx);
            return false;
        }

        if !file_path.is_file() {
            log::warn!("Attempt to access directory on fd {}: {:?}", fd, file_path);
            Self::send_error_response(fd, HttpStatus::Forbidden, &parse_tx);
            return false;
        }

        let metadata = match std::fs::metadata(&file_path) {
            Ok(meta) => meta,
            Err(e) => {
                log::error!(
                    "Error getting metadata for fd {}: {:?}: {}",
                    fd,
                    file_path,
                    e
                );
                Self::send_error_response(fd, HttpStatus::InternalServerError, &parse_tx);
                return false;
            }
        };

        let file_size = metadata.len();
        if file_size > self.max_file_size {
            log::warn!(
                "File too large on fd {}: {:?} ({} > {})",
                fd,
                file_path,
                file_size,
                self.max_file_size
            );
            Self::send_error_response(fd, HttpStatus::PayloadTooLarge, &parse_tx);
            return false;
        }

        debug!(
            "File found on fd {}: {:?} ({} bytes)",
            fd, file_path, file_size
        );

        let content_type = self.get_content_type(file_path);

        let is_head = method == "HEAD";

        let headers = format!(
            "{}Content-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            HttpStatus::Ok.as_response_line(),
            content_type,
            file_size
        );

        if !is_head {
            if !self.open_file_for_connection(
                fd,
                file_path.to_path_buf(),
                file_size,
                is_head,
                conn_manager,
            ) {
                Self::send_error_response(fd, HttpStatus::InternalServerError, &parse_tx);
                return false;
            }
        } else {
            debug!("HEAD request on fd {} for {:?}", fd, file_path);
        }

        if let Err(e) = parse_tx.send((fd, headers.clone().into_bytes())) {
            log::error!("Failed to send parsed response for fd {}: {}", fd, e);
            return false;
        }

        debug!("Headers prepared for fd {}: {} bytes", fd, headers.len());
        true
    }

    fn open_file_for_connection(
        &self,
        fd: i32,
        file_path: PathBuf,
        file_size: u64,
        is_head: bool,
        conn_manager: Arc<ConnectionManager>,
    ) -> bool {
        match std::fs::File::open(&file_path) {
            Ok(file) => {
                if !conn_manager.set_file_for_connection(fd, file, file_size, is_head) {
                    log::error!("Failed to set file for connection fd {}", fd);
                    return false;
                }
                debug!("File opened for fd {}: {} bytes", fd, file_size);
                true
            }
            Err(e) => {
                log::error!("Error opening file for fd {}: {:?}: {}", fd, file_path, e);
                false
            }
        }
    }

    fn get_content_type(&self, file_path: &PathBuf) -> &'static str {
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

    fn send_error_response(fd: i32, status: HttpStatus, parse_tx: &mpsc::Sender<(i32, Vec<u8>)>) {
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
            _ => "<html><body><h1>Undocumented error</h1></body></html>",
        };

        let response = format!(
            "{}Content-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            status.as_response_line(),
            body.len(),
            body
        );

        if let Err(e) = parse_tx.send((fd, response.into_bytes())) {
            log::error!("Failed to send error response for fd {}: {}", fd, e);
        } else {
            debug!("Error response prepared for fd {}: {}", fd, status.code());
        }
    }
}

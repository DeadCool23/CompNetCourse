use log::{debug, error, info};
use std::io::{Read, Seek, Write};
use std::sync::Arc;
use std::sync::mpsc;

use super::connection::ConnectionStage;
use super::connection_manager::ConnectionManager;
use super::request_parser::RequestParser;

pub struct ConnectionHandler {
    connection_manager: Arc<ConnectionManager>,
    request_parser: RequestParser,
    parse_tx: mpsc::Sender<(i32, Vec<u8>)>,
}

impl ConnectionHandler {
    pub fn new(
        connection_manager: Arc<ConnectionManager>,
        request_parser: RequestParser,
        parse_tx: mpsc::Sender<(i32, Vec<u8>)>,
    ) -> Self {
        Self {
            connection_manager,
            request_parser,
            parse_tx,
        }
    }

    pub fn handle_readable_connection(&self, fd: i32) {
        self.connection_manager
            .with_connection(fd, |conn| match conn.stage {
                ConnectionStage::Recv => self.handle_recv_stage(fd, conn),
                _ => {}
            });
    }

    fn handle_recv_stage(&self, fd: i32, conn: &mut super::connection::Connection) {
        let bytes_read = match conn
            .stream
            .read(&mut conn.request_buffer[conn.request_len..])
        {
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
        if Self::contains_double_newline(buffer_slice) {
            debug!(
                "Full request received on fd {} ({} bytes)",
                fd, conn.request_len
            );

            let request_data = buffer_slice[..conn.request_len].to_vec();
            let parse_tx = self.parse_tx.clone();
            let conn_manager = Arc::clone(&self.connection_manager);

            conn.request_len = 0;
            conn.stage = ConnectionStage::Parse;

            let parser = self.request_parser.clone();
            std::thread::spawn(move || {
                parser.parse_request_in_thread(fd, request_data, parse_tx, conn_manager);
            });
        }
    }

    pub fn handle_writable_connection(&self, fd: i32) {
        self.connection_manager
            .with_connection(fd, |conn| match conn.stage {
                ConnectionStage::SendHeaders => self.handle_send_headers_stage(fd, conn),
                ConnectionStage::SendFile => self.handle_send_file_stage(fd, conn),
                _ => {}
            });
    }

    fn handle_send_headers_stage(&self, fd: i32, conn: &mut super::connection::Connection) {
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

    fn handle_send_file_stage(&self, fd: i32, conn: &mut super::connection::Connection) {
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
            log::warn!("No file to send on fd {}", fd);
            conn.stage = ConnectionStage::Close;
        }
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
}

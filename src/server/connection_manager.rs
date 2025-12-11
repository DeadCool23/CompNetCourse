use std::collections::HashMap;
use std::net::{TcpListener, TcpStream};
use std::os::unix::io::RawFd;
use std::sync::{Arc, Mutex};

use crate::server::config::ServerConfig;
use crate::server::connection::{Connection, ConnectionStage};

pub struct ConnectionManager {
    connections: Arc<Mutex<HashMap<RawFd, Connection>>>,
    pub listener: TcpListener,
    max_connections: usize,
}

#[allow(dead_code)]
impl ConnectionManager {
    pub fn new(listener: TcpListener) -> Self {
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
            listener,
            max_connections: 1000,
        }
    }

    pub fn with_config(listener: TcpListener, config: &ServerConfig) -> Self {
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
            listener,
            max_connections: config.max_connections,
        }
    }

    pub fn add_connection(&self, stream: TcpStream) -> bool {
        let mut connections = self.connections.lock().unwrap();
        if connections.len() >= self.max_connections {
            return false;
        }
        let connection = Connection::new(stream);
        let fd = connection.fd;
        connections.insert(fd, connection);
        true
    }

    pub fn remove_connection(&self, fd: RawFd) -> Option<Connection> {
        let mut connections = self.connections.lock().unwrap();
        connections.remove(&fd)
    }

    pub fn with_connection<F, R>(&self, fd: RawFd, f: F) -> Option<R>
    where
        F: FnOnce(&mut Connection) -> R,
    {
        let mut connections = self.connections.lock().unwrap();
        if let Some(conn) = connections.get_mut(&fd) {
            Some(f(conn))
        } else {
            None
        }
    }

    pub fn get_connections_for_select(&self) -> (Vec<RawFd>, Vec<RawFd>) {
        let connections = self.connections.lock().unwrap();
        let mut read_fds = Vec::new();
        let mut write_fds = Vec::new();

        for (fd, conn) in connections.iter() {
            match conn.stage {
                ConnectionStage::Recv | ConnectionStage::Parse => {
                    read_fds.push(*fd);
                }
                ConnectionStage::SendHeaders | ConnectionStage::SendFile => {
                    write_fds.push(*fd);
                }
                ConnectionStage::Close => {}
            }
        }

        (read_fds, write_fds)
    }

    pub fn get_closed_connections(&self) -> Vec<RawFd> {
        let connections = self.connections.lock().unwrap();
        connections
            .iter()
            .filter(|(_, conn)| matches!(conn.stage, ConnectionStage::Close))
            .map(|(fd, _)| *fd)
            .collect()
    }

    pub fn get_connections_count(&self) -> usize {
        let connections = self.connections.lock().unwrap();
        connections.len()
    }

    pub fn set_file_for_connection(
        &self,
        fd: RawFd,
        file: std::fs::File,
        file_size: u64,
        is_head: bool,
    ) -> bool {
        let mut connections = self.connections.lock().unwrap();
        if let Some(conn) = connections.get_mut(&fd) {
            conn.file = Some(file);
            conn.file_size = file_size;
            conn.is_head = is_head;
            true
        } else {
            false
        }
    }
}

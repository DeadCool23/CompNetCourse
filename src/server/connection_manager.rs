use libc::fd_set;
use std::collections::HashMap;
use std::net::{TcpListener, TcpStream};
use std::os::unix::io::{AsRawFd, RawFd};
use std::sync::{Arc, Mutex};

use crate::server::config::ServerConfig;

pub struct ConnectionManager {
    pub connections: Arc<Mutex<HashMap<RawFd, TcpStream>>>,
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
        let fd = stream.as_raw_fd();
        connections.insert(fd, stream);
        true
    }

    pub fn remove_connection(&self, fd: RawFd) {
        let mut connections = self.connections.lock().unwrap();
        connections.remove(&fd);
    }

    pub fn get_connections_fds(&self) -> Vec<RawFd> {
        let connections = self.connections.lock().unwrap();
        connections.keys().cloned().collect()
    }

    pub fn get_stream(&self, fd: RawFd) -> Option<TcpStream> {
        let mut connections = self.connections.lock().unwrap();
        connections.remove(&fd)
    }
}

#[allow(unsafe_op_in_unsafe_fn)]
pub unsafe fn fd_isset(fd: RawFd, set: *mut fd_set) -> bool {
    #[cfg(target_os = "linux")]
    {
        use libc::FD_ISSET;
        FD_ISSET(fd, set)
    }
    
    #[cfg(not(target_os = "linux"))]
    {
        let set_ref = &*set;
        let fd = fd as usize;
        let bits_per_element = std::mem::size_of::<libc::c_ulong>() * 8;
        let word = fd / bits_per_element;
        let bit = fd % bits_per_element;
        
        if word >= set_ref.fds_bits.len() {
            false
        } else {
            (set_ref.fds_bits[word] & (1 << bit)) != 0
        }
    }
}
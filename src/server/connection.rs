use std::fs::File;
use std::net::TcpStream;
use std::os::unix::io::{AsRawFd, RawFd};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionStage {
    Recv,
    Parse,
    SendHeaders,
    SendFile,
    Close,
}

#[derive(Debug)]
pub struct Connection {
    pub fd: RawFd,
    pub stream: TcpStream,
    pub stage: ConnectionStage,
    pub request_buffer: Vec<u8>,
    pub request_len: usize,
    pub file: Option<File>,
    pub file_size: u64,
    pub file_sent: u64,
    pub headers: Vec<u8>,
    pub headers_sent: usize,
    pub is_head: bool,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Self {
        let fd = stream.as_raw_fd();

        Self {
            fd,
            stream,
            stage: ConnectionStage::Recv,
            request_buffer: vec![0u8; 8192],
            request_len: 0,
            file: None,
            file_size: 0,
            file_sent: 0,
            headers: Vec::new(),
            headers_sent: 0,
            is_head: false,
        }
    }
}

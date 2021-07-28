//! Shuttle's implementation of [`tokio::fs`].

// pub use tokio::fs::*;

pub use std::fs::*;

use std::io::{self, Read, SeekFrom, Write};
use std::path::Path;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::ReadBuf;

/// TODO
pub async fn read_dir<P: AsRef<Path>>(path: P) -> io::Result<ReadDir>{
    println!("Performed read_dir");
    match std::fs::read_dir(path) {
        Ok(inner) => Ok(ReadDir { inner }),
        Err(e) => Err(e),
    }
}

/// TODO
pub async fn read_to_string<P: AsRef<Path>>(path: P) -> io::Result<String>{
    println!("Performed read_to_string");
    std::fs::read_to_string(path)
}

/// Wrapper around std::fs::ReadDir
#[derive(Debug)]
pub struct ReadDir {
    inner: std::fs::ReadDir,
}

impl ReadDir {
    /// Wrapper around std::fs::ReadDir::next
    pub async fn next_entry(&mut self) -> io::Result<Option<DirEntry>> {
        match self.inner.next() {
            None => Ok(None),
            Some(Ok(dir)) => Ok(Some(dir)),
            Some(Err(e)) => Err(e),
        }
    }
}

/// TODO
#[derive(Debug)]
pub struct File {
    inner: std::fs::File,
}

impl File {
    /// Wrapper around std::fs::File::create
    pub async fn create<P: AsRef<Path>>(path: P) -> io::Result<File> {
        println!("Performed create");
        match std::fs::File::create(path) {
            Ok(inner) => Ok(File { inner }),
            Err(e) => Err(e),
        }
    }

    /// Wrapper around std::fs::File::open
    pub async fn open<P: AsRef<Path>>(path: P) -> io::Result<File> {
        println!("Performed open");
        match std::fs::File::open(path) {
            Ok(inner) => Ok(File { inner }),
            Err(e) => Err(e),
        }
    }

    /// Creates a shuttle::tokio::fs::File given a std::fs::File
    pub fn from_std(inner: std::fs::File) -> File {
        println!("Performed from_std");
        File { inner }
    }

    /// Get the inner std::fs::File
    pub fn try_into_std(self) -> Result<std::fs::File, Self> {
        println!("Performed try_into_std");
        Ok(self.inner)
    }

    /// Wrapper around std:;fs::File::metadata
    pub async fn metadata(&self) -> io::Result<Metadata>{
        println!("Performed metadata");
        self.inner.metadata()
    }
}

impl crate::tokio::io::AsyncWrite for File {
    fn poll_write(mut self: Pin<&mut Self>, _: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize, io::Error>> {
        println!("Performed poll_write");
        Poll::Ready(self.inner.write(buf))
    }

    fn poll_flush(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        println!("Performed poll_flush");
        Poll::Ready(self.inner.flush())
    }

    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        println!("Performed poll_shutdown");
        // NOTE: Just ignore this for now
        Poll::Ready(Ok(()))
    }
}

impl crate::tokio::io::AsyncRead for File {
    fn poll_read(mut self: Pin<&mut Self>, _: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<Result<(), io::Error>> {
        println!("Performed poll_read");
        Poll::Ready(self.inner.read(buf.initialized_mut()).map(|_| {}))
    }
}

impl crate::tokio::io::AsyncSeek for File {
    fn start_seek(self: Pin<&mut Self>, mut _pos: SeekFrom) -> Result<(), io::Error> {
        // self.inner.se
        println!("Performed start_seek");
        Ok(())
    }

    fn poll_complete(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<u64, std::io::Error>> {
        println!("Performed poll_complete");
        Poll::Ready(Ok(0))
    }
}
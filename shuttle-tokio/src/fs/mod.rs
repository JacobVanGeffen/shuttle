//! Shuttle's implementation of [`tokio::fs`].

// pub use tokio::fs::*;

pub use std::fs::*;

use std::io::{self, Read, SeekFrom, Write};
use std::path::Path;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::ReadBuf;

#[cfg(test)]
use shuttle::{asynch, check_dfs};

/// TODO
pub async fn create_dir_all<P: AsRef<Path>>(path: P) -> io::Result<()> {
    std::fs::create_dir_all(path)
}

/// TODO
pub async fn read_dir<P: AsRef<Path>>(path: P) -> io::Result<ReadDir> {
    match std::fs::read_dir(path) {
        Ok(inner) => Ok(ReadDir { inner }),
        Err(e) => Err(e),
    }
}

/// TODO
pub async fn read_to_string<P: AsRef<Path>>(path: P) -> io::Result<String> {
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
        match std::fs::File::create(path) {
            Ok(inner) => Ok(File { inner }),
            Err(e) => Err(e),
        }
    }

    /// Wrapper around std::fs::File::open
    pub async fn open<P: AsRef<Path>>(path: P) -> io::Result<File> {
        match std::fs::File::open(path) {
            Ok(inner) => {
                Ok(File { inner })
            }
            Err(e) => {
                Err(e)
            }
        }
    }

    /// Creates a shuttle::tokio::fs::File given a std::fs::File
    pub fn from_std(inner: std::fs::File) -> File {
        File { inner }
    }

    /// Get the inner std::fs::File
    pub fn try_into_std(self) -> Result<std::fs::File, Self> {
        Ok(self.inner)
    }

    /// Wrapper around std:;fs::File::metadata
    pub async fn metadata(&self) -> io::Result<Metadata> {
        self.inner.metadata()
    }
}

impl tokio::io::AsyncWrite for File {
    fn poll_write(mut self: Pin<&mut Self>, _: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize, io::Error>> {
        Poll::Ready(self.inner.write(buf))
    }

    fn poll_flush(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        Poll::Ready(self.inner.flush())
    }

    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        // NOTE: Just ignore this for now
        Poll::Ready(Ok(()))
    }
}

impl tokio::io::AsyncRead for File {
    fn poll_read(mut self: Pin<&mut Self>, _: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<Result<(), io::Error>> {
        let read_buf = buf.initialize_unfilled();
        let res = self.inner.read(read_buf);
        if let Ok(n_bytes) = res {
            buf.advance(n_bytes);
        }
        Poll::Ready(res.map(|_| {}))
    }
}

impl tokio::io::AsyncSeek for File {
    fn start_seek(self: Pin<&mut Self>, mut _pos: SeekFrom) -> Result<(), io::Error> {
        Ok(())
    }

    fn poll_complete(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<u64, std::io::Error>> {
        Poll::Ready(Ok(0))
    }
}

#[test]
fn test_file_read() {
    use tokio::io::AsyncReadExt;
    check_dfs(
        || {
            asynch::block_on(async move {
                let mut file = File::open("test-file.txt").await.unwrap();
                let mut buf = Vec::new();
                let n_bytes = file.read_to_end(&mut buf).await.unwrap();
                let str = std::str::from_utf8(&buf[..n_bytes]).unwrap();
                assert_eq!(str, "Example text")
            });
        },
        None,
    );
}

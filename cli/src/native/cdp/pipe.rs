//! Linux anonymous CDP transport. Chrome receives commands on fd 3 and emits
//! responses on fd 4. Frames are UTF-8 JSON followed by NUL, not WebSocket frames.
//! Protocol sources:
//! https://github.com/puppeteer/puppeteer/blob/main/packages/puppeteer-core/src/node/PipeTransport.ts
//! https://github.com/chromium/chromium/blob/main/content/browser/devtools/devtools_pipe_handler.cc
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::process::CommandExt;
use std::pin::Pin;
use std::process::Command;
use std::task::{ready, Context, Poll};

use futures_util::{Sink, Stream};
use tokio::io::unix::AsyncFd;
use tokio_tungstenite::tungstenite::{Error, Message};

pub const MAX_FRAME_BYTES: usize = 16 * 1024 * 1024;

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn pipe_pair() -> io::Result<(OwnedFd, OwnedFd)> {
    let mut fds = [-1; 2];
    // SAFETY: writable array has room for both descriptors.
    if unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) } != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: pipe2 returned two unique owned descriptors.
    Ok(unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) })
}

fn child_descriptor(fd: OwnedFd) -> io::Result<OwnedFd> {
    // Reserve descriptors above 4 before fork, avoiding dup2 source collisions.
    // SAFETY: fcntl duplicates a valid descriptor and does not retain references.
    let duplicate = unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 5) };
    if duplicate < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: duplicate is a newly allocated descriptor owned by this function.
    Ok(unsafe { OwnedFd::from_raw_fd(duplicate) })
}

fn nonblocking(fd: &OwnedFd) -> io::Result<()> {
    // SAFETY: both calls operate on a live descriptor.
    let flags = unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_GETFL) };
    if flags < 0
        || unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Two anonymous pipes, without a listening socket or discoverable CDP endpoint.
pub struct PreparedPipe {
    read: OwnedFd,
    write: OwnedFd,
    child: Option<(OwnedFd, OwnedFd)>,
}

impl PreparedPipe {
    /// Borrow descriptor identities for a fresh launch proof; does not duplicate
    /// or transfer either endpoint. The transport retains ownership after conversion.
    pub(crate) fn parent_descriptors(&self) -> (i32, i32) {
        (self.read.as_raw_fd(), self.write.as_raw_fd())
    }
    pub fn new() -> io::Result<Self> {
        let (child_read, write) = pipe_pair()?;
        let (read, child_write) = pipe_pair()?;
        nonblocking(&read)?;
        nonblocking(&write)?;
        Ok(Self {
            read,
            write,
            child: Some((
                child_descriptor(child_read)?,
                child_descriptor(child_write)?,
            )),
        })
    }

    /// Configure before spawn. Drop `command` immediately after spawn (including
    /// failure): its closure owns the parent's copies of the child endpoints.
    /// Child endpoints remain blocking; parent endpoints are nonblocking/CLOEXEC.
    pub fn configure_command(&mut self, command: &mut Command) -> io::Result<()> {
        let (read, write) = self
            .child
            .take()
            .ok_or_else(|| invalid("CDP pipe already configured"))?;
        // SAFETY: closure calls only async-signal-safe dup2 and errno capture;
        // descriptors were moved above 4 before fork. No locks or allocation.
        unsafe {
            command.pre_exec(move || {
                if libc::dup2(read.as_raw_fd(), 3) < 0 || libc::dup2(write.as_raw_fd(), 4) < 0 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
        Ok(())
    }

    /// Register parent endpoints with the current Tokio reactor after spawn.
    pub fn into_transport(self) -> io::Result<PipeTransport> {
        if self.child.is_some() {
            return Err(invalid("CDP pipe must be configured before conversion"));
        }
        Ok(PipeTransport {
            read: Some(AsyncFd::new(self.read)?),
            write: Some(AsyncFd::new(self.write)?),
            incoming: Vec::new(),
            scanned: 0,
            outgoing: Vec::new(),
            written: 0,
            failed: false,
        })
    }
}

/// Bounded duplex adapter for the existing CDP Sink/Stream client. Any malformed
/// incoming frame poisons both directions. No ping/pong protocol exists here.
pub struct PipeTransport {
    read: Option<AsyncFd<OwnedFd>>,
    write: Option<AsyncFd<OwnedFd>>,
    incoming: Vec<u8>,
    scanned: usize,
    outgoing: Vec<u8>,
    written: usize,
    failed: bool,
}

impl PipeTransport {
    fn fail(&mut self, error: io::Error) -> Error {
        self.failed = true;
        self.read = None;
        self.write = None;
        self.incoming.clear();
        self.outgoing.clear();
        Error::Io(error)
    }
}

impl Stream for PipeTransport {
    type Item = Result<Message, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        loop {
            if this.failed || this.read.is_none() {
                return Poll::Ready(None);
            }
            if let Some(offset) = this.incoming[this.scanned..].iter().position(|b| *b == 0) {
                let end = this.scanned + offset;
                let bytes: Vec<u8> = this.incoming.drain(..=end).collect();
                this.scanned = 0;
                return match std::str::from_utf8(&bytes[..end]) {
                    Ok(text) if serde_json::from_str::<serde_json::Value>(text).is_ok() => {
                        Poll::Ready(Some(Ok(Message::Text(text.to_owned()))))
                    }
                    _ => Poll::Ready(Some(Err(
                        this.fail(invalid("Invalid UTF-8 JSON CDP pipe frame"))
                    ))),
                };
            }
            this.scanned = this.incoming.len();
            if this.incoming.len() > MAX_FRAME_BYTES {
                return Poll::Ready(Some(
                    Err(this.fail(invalid("CDP pipe frame exceeds limit"))),
                ));
            }
            let mut buffer = [0u8; 8192];
            let capacity = buffer.len().min(MAX_FRAME_BYTES + 1 - this.incoming.len());
            let fd = this.read.as_ref().expect("checked read endpoint");
            let mut guard = match ready!(fd.poll_read_ready(cx)) {
                Ok(guard) => guard,
                Err(error) => return Poll::Ready(Some(Err(this.fail(error)))),
            };
            let result = guard.try_io(|inner| {
                // SAFETY: buffer is writable for capacity bytes and fd is live.
                let count =
                    unsafe { libc::read(inner.as_raw_fd(), buffer.as_mut_ptr().cast(), capacity) };
                if count < 0 {
                    Err(io::Error::last_os_error())
                } else {
                    Ok(count as usize)
                }
            });
            match result {
                Err(_) => continue,
                Ok(Err(error)) => return Poll::Ready(Some(Err(this.fail(error)))),
                Ok(Ok(0)) if !this.incoming.is_empty() => {
                    return Poll::Ready(Some(Err(this.fail(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "Truncated CDP pipe frame",
                    )))));
                }
                Ok(Ok(0)) => {
                    this.read = None;
                    this.write = None;
                    return Poll::Ready(None);
                }
                Ok(Ok(count)) => this.incoming.extend_from_slice(&buffer[..count]),
            }
        }
    }
}

impl Sink<Message> for PipeTransport {
    type Error = Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        self.poll_flush(cx)
    }

    fn start_send(self: Pin<&mut Self>, item: Message) -> Result<(), Error> {
        let this = self.get_mut();
        if this.failed || this.write.is_none() {
            return Err(Error::ConnectionClosed);
        }
        if !this.outgoing.is_empty() {
            return Err(this.fail(invalid("CDP pipe send called without readiness")));
        }
        let Message::Text(text) = item else {
            return Err(this.fail(invalid(
                "CDP pipe accepts text only; no WebSocket control frames",
            )));
        };
        if text.len() > MAX_FRAME_BYTES
            || text.as_bytes().contains(&0)
            || serde_json::from_str::<serde_json::Value>(&text).is_err()
        {
            return Err(this.fail(invalid("Invalid or oversized outgoing CDP pipe JSON")));
        }
        this.outgoing = text.into_bytes();
        this.outgoing.push(0);
        this.written = 0;
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        let this = self.get_mut();
        if this.failed || this.write.is_none() {
            return Poll::Ready(Err(Error::ConnectionClosed));
        }
        while this.written < this.outgoing.len() {
            let fd = this.write.as_ref().expect("checked write endpoint");
            let mut guard = match ready!(fd.poll_write_ready(cx)) {
                Ok(guard) => guard,
                Err(error) => return Poll::Ready(Err(this.fail(error))),
            };
            let result = guard.try_io(|inner| {
                let bytes = &this.outgoing[this.written..];
                // SAFETY: bytes remains live during the synchronous write.
                let count =
                    unsafe { libc::write(inner.as_raw_fd(), bytes.as_ptr().cast(), bytes.len()) };
                if count < 0 {
                    Err(io::Error::last_os_error())
                } else {
                    Ok(count as usize)
                }
            });
            match result {
                Err(_) => continue,
                Ok(Err(error)) => return Poll::Ready(Err(this.fail(error))),
                Ok(Ok(0)) => {
                    return Poll::Ready(Err(this.fail(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "CDP pipe write returned zero",
                    ))))
                }
                Ok(Ok(count)) => this.written += count,
            }
        }
        this.outgoing.clear();
        this.written = 0;
        Poll::Ready(Ok(()))
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        ready!(self.as_mut().poll_flush(cx))?;
        self.write = None;
        self.read = None;
        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::{SinkExt, StreamExt};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::unix::pipe::{Receiver, Sender};

    fn fixture() -> (PipeTransport, Receiver, Sender) {
        let mut prepared = PreparedPipe::new().unwrap();
        let (read, write) = prepared.child.take().unwrap();
        nonblocking(&read).unwrap();
        nonblocking(&write).unwrap();
        (
            prepared.into_transport().unwrap(),
            Receiver::from_owned_fd(read).unwrap(),
            Sender::from_owned_fd(write).unwrap(),
        )
    }

    #[tokio::test]
    async fn pipe_client_dispatches_response_and_closes_on_drop() {
        let (transport, mut commands, mut replies) = fixture();
        let client = crate::native::cdp::client::CdpClient::connect_pipe(transport);
        let peer = tokio::spawn(async move {
            let mut bytes = Vec::new();
            loop {
                let byte = commands.read_u8().await.unwrap();
                if byte == 0 {
                    break;
                }
                bytes.push(byte);
            }
            let command: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(command["method"], "Browser.getVersion");
            let response = serde_json::json!({"id":command["id"], "result":{"product":"fixture"}});
            replies
                .write_all(response.to_string().as_bytes())
                .await
                .unwrap();
            replies.write_all(&[0]).await.unwrap();
            let mut byte = [0];
            assert_eq!(commands.read(&mut byte).await.unwrap(), 0);
        });
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            client.send_command("Browser.getVersion", None, None),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(result["product"], "fixture");
        drop(client);
        tokio::time::timeout(std::time::Duration::from_secs(2), peer)
            .await
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn pipe_fragmented_coalesced_and_clean_eof() {
        let (mut transport, _commands, mut replies) = fixture();
        replies.write_all(b"{\"id\":").await.unwrap();
        let receive = tokio::spawn(async move {
            let first = transport.next().await.unwrap().unwrap();
            let second = transport.next().await.unwrap().unwrap();
            assert_eq!(first, Message::Text("{\"id\":1}".into()));
            assert_eq!(second, Message::Text("{\"id\":2}".into()));
            assert!(transport.next().await.is_none());
        });
        replies.write_all(b"1}\0{\"id\":2}\0").await.unwrap();
        drop(replies);
        receive.await.unwrap();
    }

    #[tokio::test]
    async fn pipe_truncated_invalid_utf8_and_json_are_terminal() {
        for bytes in [b"{\"id\":1}".as_slice(), b"\xff\0", b"not json\0"] {
            let (mut transport, _commands, mut replies) = fixture();
            replies.write_all(bytes).await.unwrap();
            drop(replies);
            assert!(transport.next().await.unwrap().is_err());
            assert!(transport.next().await.is_none());
            assert!(transport.send(Message::Text("{}".into())).await.is_err());
        }
    }

    #[tokio::test]
    async fn pipe_oversized_incoming_is_terminal() {
        let (mut transport, _commands, mut replies) = fixture();
        let producer = tokio::spawn(async move {
            let bytes = vec![b' '; MAX_FRAME_BYTES + 1];
            replies.write_all(&bytes).await.unwrap();
        });
        assert!(transport.next().await.unwrap().is_err());
        assert!(transport.next().await.is_none());
        producer.await.unwrap();
    }

    #[tokio::test]
    async fn pipe_exact_limit_and_write_backpressure() {
        let (mut transport, mut commands, mut replies) = fixture();
        let text = format!("\"{}\"", "a".repeat(MAX_FRAME_BYTES - 2));
        let expected = text.clone();
        let peer = tokio::spawn(async move {
            let mut bytes = vec![0; MAX_FRAME_BYTES + 1];
            commands.read_exact(&mut bytes).await.unwrap();
            assert_eq!(&bytes[..MAX_FRAME_BYTES], expected.as_bytes());
            assert_eq!(bytes[MAX_FRAME_BYTES], 0);
            replies.write_all(&bytes).await.unwrap();
        });
        transport.send(Message::Text(text.clone())).await.unwrap();
        assert_eq!(
            transport.next().await.unwrap().unwrap(),
            Message::Text(text)
        );
        peer.await.unwrap();
    }

    #[tokio::test]
    async fn pipe_writes_nul_and_rejects_unsafe_messages() {
        let (mut transport, mut commands, _replies) = fixture();
        transport
            .send(Message::Text("{\"id\":7}".into()))
            .await
            .unwrap();
        let mut bytes = [0; 9];
        commands.read_exact(&mut bytes).await.unwrap();
        assert_eq!(&bytes, b"{\"id\":7}\0");
        for message in [
            Message::Text("{}\0{}".into()),
            Message::Text(" ".repeat(MAX_FRAME_BYTES + 1)),
            Message::Ping(Vec::new()),
        ] {
            let (mut transport, _commands, _replies) = fixture();
            assert!(transport.send(message).await.is_err());
            assert!(transport.next().await.is_none());
        }
    }

    #[tokio::test]
    async fn pipe_child_mapping_and_parent_descriptor_cleanup() {
        let mut prepared = PreparedPipe::new().unwrap();
        let (read, write) = prepared.child.as_ref().unwrap();
        for fd in [read, write] {
            assert!(fd.as_raw_fd() >= 5);
            // SAFETY: querying valid descriptors, no mutation.
            assert_eq!(
                unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_GETFL) } & libc::O_NONBLOCK,
                0
            );
            assert_ne!(
                unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_GETFD) } & libc::FD_CLOEXEC,
                0
            );
        }
        let mut command = Command::new("/bin/sh");
        // This synthetic child echoes fd 3 to fd 4, never launching a browser.
        command.args(["-c", "exec cat <&3 >&4"]);
        prepared.configure_command(&mut command).unwrap();
        let mut child = command.spawn().unwrap();
        drop(command);
        let mut transport = prepared.into_transport().unwrap();
        transport
            .send(Message::Text("{\"id\":9}".into()))
            .await
            .unwrap();
        assert_eq!(
            transport.next().await.unwrap().unwrap(),
            Message::Text("{\"id\":9}".into())
        );
        // Closing parent writer must deliver EOF to child: no retained command end.
        transport.close().await.unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success());
                break;
            }
            if std::time::Instant::now() >= deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("Synthetic pipe child retained a pipe endpoint");
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }

    #[tokio::test]
    async fn pipe_spawn_failure_releases_child_endpoints() {
        let mut prepared = PreparedPipe::new().unwrap();
        let mut command = Command::new("/nonexistent-agent-browser-pipe-test-executable");
        prepared.configure_command(&mut command).unwrap();
        assert!(command.spawn().is_err());
        drop(command);
        let mut transport = prepared.into_transport().unwrap();
        assert!(
            tokio::time::timeout(std::time::Duration::from_secs(1), transport.next())
                .await
                .expect("Child response writer leaked after spawn failure")
                .is_none()
        );
    }
}

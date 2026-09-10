//! Bounded local X socket peer inspection, not complete display custody proof.
//! The caller must derive `expected_uid` from an authoritative route identity,
//! never from request-supplied identity claims. No X protocol data is sent.

use serde_json::Value;

/// Verify a local display's kernel-reported socket owner and stable process.
/// This does not authenticate X access, attest the executable, or grant access.
pub fn verify_display_owner(display: &str, expected_uid: u32) -> Result<Value, String> {
    #[cfg(target_os = "linux")]
    {
        linux::verify(display, expected_uid)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (display, expected_uid);
        Err("display_owner_proof_unsupported: Linux peer credentials are required".into())
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use serde_json::{json, Value};
    use std::{
        io,
        mem::{size_of, zeroed},
        os::fd::{AsRawFd, FromRawFd, OwnedFd},
        time::{Duration, Instant},
    };

    const CONNECT_BUDGET: Duration = Duration::from_millis(250);

    fn failure(reason: &str) -> String {
        format!("display_owner_proof_failed: {reason}")
    }

    fn socket_path(display: &str) -> Result<String, String> {
        let local = display
            .strip_prefix(':')
            .or_else(|| display.strip_prefix("unix:"))
            .ok_or_else(|| failure("only local :N or unix:N displays are supported"))?;
        let mut parts = local.split('.');
        let number = parts.next().unwrap_or_default();
        let screen = parts.next();
        let numeric = |part: &str| {
            !part.is_empty()
                && part.bytes().all(|byte| byte.is_ascii_digit())
                && part.parse::<u32>().is_ok()
        };
        if !numeric(number) || screen.is_some_and(|part| !numeric(part)) || parts.next().is_some() {
            return Err(failure("invalid local display number"));
        }
        // Normalize decimal aliases so DISPLAY cannot select an arbitrary path.
        let number = number
            .parse::<u32>()
            .map_err(|_| failure("invalid display"))?;
        Ok(format!("/tmp/.X11-unix/X{number}"))
    }

    fn connect(path: &str, deadline: Instant) -> Result<OwnedFd, String> {
        if Instant::now() >= deadline {
            return Err(failure("local display socket connection timed out"));
        }
        // SAFETY: socket takes scalar arguments and returns a new owned fd.
        let raw = unsafe {
            libc::socket(
                libc::AF_UNIX,
                libc::SOCK_STREAM | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK,
                0,
            )
        };
        if raw < 0 {
            return Err(failure("cannot create inspection socket"));
        }
        // SAFETY: raw is a new valid fd, owned exclusively here.
        let fd = unsafe { OwnedFd::from_raw_fd(raw) };
        // SAFETY: an all-zero sockaddr_un is valid initialization.
        let mut address: libc::sockaddr_un = unsafe { zeroed() };
        address.sun_family = libc::AF_UNIX as libc::sa_family_t;
        if path.len() >= address.sun_path.len() || path.as_bytes().contains(&0) {
            return Err(failure("invalid socket path"));
        }
        for (slot, byte) in address.sun_path.iter_mut().zip(path.bytes()) {
            *slot = byte as libc::c_char;
        }
        // SAFETY: address points to an initialized sockaddr_un of supplied size.
        let connected = unsafe {
            libc::connect(
                fd.as_raw_fd(),
                (&address as *const libc::sockaddr_un).cast(),
                size_of::<libc::sockaddr_un>() as libc::socklen_t,
            )
        };
        if connected == 0 && Instant::now() < deadline {
            return Ok(fd);
        }
        // AF_UNIX EAGAIN means a full listen queue, not a pending connection.
        // Fail closed instead of retrying or treating writable as connected.
        if io::Error::last_os_error().raw_os_error() != Some(libc::EINPROGRESS) {
            return Err(failure("local display socket connection unavailable"));
        }
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            let millis = remaining.as_millis().min(250) as libc::c_int;
            if millis == 0 {
                return Err(failure("local display socket connection timed out"));
            }
            let mut pollfd = libc::pollfd {
                fd: fd.as_raw_fd(),
                events: libc::POLLOUT,
                revents: 0,
            };
            // SAFETY: one valid writable pollfd is supplied for this bounded wait.
            let result = unsafe { libc::poll(&mut pollfd, 1, millis) };
            if result < 0 && io::Error::last_os_error().kind() == io::ErrorKind::Interrupted {
                continue;
            }
            if result <= 0 || pollfd.revents & libc::POLLNVAL != 0 {
                return Err(failure("local display socket connection did not complete"));
            }
            let mut error: libc::c_int = 0;
            let mut length = size_of::<libc::c_int>() as libc::socklen_t;
            // SAFETY: writable error storage and its exact size are supplied.
            let result = unsafe {
                libc::getsockopt(
                    fd.as_raw_fd(),
                    libc::SOL_SOCKET,
                    libc::SO_ERROR,
                    (&mut error as *mut libc::c_int).cast(),
                    &mut length,
                )
            };
            if result != 0
                || length as usize != size_of::<libc::c_int>()
                || error != 0
                || pollfd.revents & libc::POLLOUT == 0
                || Instant::now() >= deadline
            {
                return Err(failure("local display socket connection failed"));
            }
            return Ok(fd);
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct Peer {
        pid: i32,
        uid: u32,
        gid: u32,
    }

    fn peer(fd: &OwnedFd) -> Result<Peer, String> {
        // SAFETY: all-zero ucred is valid storage for getsockopt output.
        let mut credentials: libc::ucred = unsafe { zeroed() };
        let mut length = size_of::<libc::ucred>() as libc::socklen_t;
        // SAFETY: writable credential storage and its exact size are supplied.
        let result = unsafe {
            libc::getsockopt(
                fd.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                (&mut credentials as *mut libc::ucred).cast(),
                &mut length,
            )
        };
        if result != 0 || length as usize != size_of::<libc::ucred>() {
            return Err(failure("kernel peer credentials unavailable"));
        }
        Ok(Peer {
            pid: credentials.pid,
            uid: credentials.uid,
            gid: credentials.gid,
        })
    }

    fn parse_start_ticks(stat: &str) -> Result<u64, String> {
        let (_, fields) = stat
            .rsplit_once(") ")
            .ok_or_else(|| failure("invalid peer process stat"))?;
        let fields: Vec<_> = fields.split_whitespace().collect();
        if matches!(fields.first(), Some(&"Z" | &"X" | &"x")) {
            return Err(failure("peer process has exited"));
        }
        fields
            .get(19)
            .and_then(|ticks| ticks.parse::<u64>().ok())
            .filter(|ticks| *ticks > 0)
            .ok_or_else(|| failure("peer process start ticks unavailable"))
    }

    fn start_ticks(pid: i32) -> Result<u64, String> {
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat"))
            .map_err(|_| failure("peer process stat unreadable"))?;
        parse_start_ticks(&stat)
    }

    fn validate(
        expected_uid: u32,
        before: Peer,
        after: Peer,
        start_before: u64,
        start_after: u64,
    ) -> Result<(), String> {
        if before.pid <= 0 || before.uid != expected_uid {
            return Err(failure(
                "peer PID invalid or owner UID does not match route owner",
            ));
        }
        if before != after || start_before == 0 || start_before != start_after {
            return Err(failure("peer process identity changed during observation"));
        }
        Ok(())
    }

    fn observe(path: &str, expected_uid: u32) -> Result<Value, String> {
        let deadline = Instant::now() + CONNECT_BUDGET;
        let fd = connect(path, deadline)?;
        let before = peer(&fd)?;
        if before.pid <= 0 || before.uid != expected_uid {
            return Err(failure(
                "peer PID invalid or owner UID does not match route owner",
            ));
        }
        let start_before = start_ticks(before.pid)?;
        // A fresh connection brackets the observed peer with process samples.
        // Re-reading SO_PEERCRED on the same fd alone would only repeat cached
        // connect-time credentials, potentially hiding an earlier PID reuse.
        let observed_fd = connect(path, deadline)?;
        let after = peer(&observed_fd)?;
        if after.pid <= 0 || after.uid != expected_uid || after != before {
            return Err(failure("peer process identity changed during observation"));
        }
        let start_after = start_ticks(after.pid)?;
        validate(expected_uid, before, after, start_before, start_after)?;
        Ok(json!({
            "proofKind": "linux_x_socket_peer_credentials",
            "socketPath": path,
            "peerPid": before.pid,
            "peerUid": before.uid,
            "peerGid": before.gid,
            "processStartTicks": start_before,
            "expectedUid": expected_uid,
            "exclusiveOwnershipAttested": false,
        }))
    }

    pub(super) fn verify(display: &str, expected_uid: u32) -> Result<Value, String> {
        let path = socket_path(display)?;
        let mut proof = observe(&path, expected_uid)?;
        proof["display"] = json!(display);
        Ok(proof)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::os::unix::net::UnixListener;

        #[test]
        fn local_display_paths_are_strict() {
            for display in [":7", ":7.0", "unix:7", "unix:007.2"] {
                assert_eq!(socket_path(display).unwrap(), "/tmp/.X11-unix/X7");
            }
            for display in [
                "",
                ":",
                "localhost:7",
                "host:7",
                ":../7",
                ":7.",
                ":7.0.1",
                ":-1",
                ":4294967296",
                ":7\0",
            ] {
                assert!(socket_path(display).is_err(), "accepted {display:?}");
            }
        }

        #[test]
        fn observed_identity_must_match_and_remain_stable() {
            let peer = Peer {
                pid: 42,
                uid: 1001,
                gid: 1001,
            };
            assert!(validate(1001, peer, peer, 90, 90).is_ok());
            assert!(validate(1002, peer, peer, 90, 90).is_err());
            assert!(validate(1001, Peer { pid: 0, ..peer }, peer, 90, 90).is_err());
            assert!(validate(1001, peer, Peer { pid: 43, ..peer }, 90, 90).is_err());
            assert!(validate(1001, peer, Peer { uid: 1002, ..peer }, 90, 90).is_err());
            assert!(validate(1001, peer, peer, 90, 91).is_err());
            assert!(validate(1001, peer, peer, 0, 0).is_err());
        }

        #[test]
        fn process_stat_rejects_missing_dead_or_invalid_identity() {
            let fields = format!("S {} 123", ["0"; 18].join(" "));
            assert_eq!(
                parse_start_ticks(&format!("42 (name ) with spaces) {fields}")).unwrap(),
                123
            );
            assert!(parse_start_ticks("invalid").is_err());
            assert!(parse_start_ticks("42 (name) S 1").is_err());
            assert!(
                parse_start_ticks(&format!("42 (name) {}", fields.replacen('S', "Z", 1))).is_err()
            );
        }

        #[test]
        fn exhausted_connection_budget_fails_before_socket_access() {
            assert!(connect("/not-accessed", Instant::now()).is_err());
        }

        #[test]
        fn owned_fixture_socket_proves_kernel_uid_without_x_protocol() {
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = std::env::temp_dir()
                .join(format!("ab-display-proof-{}-{nonce}", std::process::id()));
            std::fs::create_dir(&root).unwrap();
            let path = root.join("fixture.sock");
            let listener = UnixListener::bind(&path).unwrap();
            // SAFETY: geteuid has no preconditions and reads only this process UID.
            let uid = unsafe { libc::geteuid() };
            let proof = observe(path.to_str().unwrap(), uid).unwrap();
            assert_eq!(proof["peerUid"], uid);
            assert_eq!(proof["peerPid"], std::process::id());
            assert!(proof["processStartTicks"].as_u64().unwrap() > 0);
            assert_eq!(proof["exclusiveOwnershipAttested"], false);
            assert!(observe(path.to_str().unwrap(), uid.wrapping_add(1)).is_err());
            drop(listener);
            assert!(observe(path.to_str().unwrap(), uid).is_err());
            std::fs::remove_file(path).unwrap();
            std::fs::remove_dir(root).unwrap();
        }
    }
}

//! Fresh, Linux-owned anonymous-pipe launch evidence. The capability is held in
//! memory and cannot be deserialized, adopted from a legacy process, or restored
//! from its diagnostic JSON. This is not isolation from a compromised OS user.
#![cfg(target_os = "linux")]

use super::handoff_custody::ProcessIdentity;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::ffi::CString;
use std::fs::{self, File, Metadata};
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

fn failure(error: impl std::fmt::Display) -> String {
    format!("owned_pipe_observation_failed:{error}")
}

fn same_inode(a: &Metadata, b: &Metadata) -> bool {
    a.dev() == b.dev() && a.ino() == b.ino()
}

fn open_at(parent: &File, name: &str, flags: i32, mode: u32) -> Result<File, String> {
    let name = CString::new(name).map_err(failure)?;
    // SAFETY: parent is live; name is NUL terminated; mode is provided for CREATE.
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            flags | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            mode,
        )
    };
    if fd < 0 {
        return Err(failure(io::Error::last_os_error()));
    }
    // SAFETY: openat returned a new descriptor owned by this File.
    Ok(unsafe { File::from_raw_fd(fd) })
}

fn private(metadata: &Metadata, directory: bool) -> Result<(), String> {
    // SAFETY: geteuid has no preconditions.
    if metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o077 != 0
        || if directory {
            !metadata.is_dir()
        } else {
            !metadata.is_file() || metadata.nlink() != 1
        }
    {
        return Err("owned_pipe_private_custody_path_required".into());
    }
    Ok(())
}

/// Acquired before spawn. Dropping it releases the cooperative profile lock,
/// but never removes the lock file (which would allow split-inode ownership).
#[derive(Debug)]
pub(crate) struct PendingLaunch {
    owner: ProcessIdentity,
    canonical_profile: PathBuf,
    profile: File,
    directory: File,
    lock: File,
    lock_name: String,
    launch_id: String,
}

impl PendingLaunch {
    pub(crate) fn acquire(profile: &Path) -> Result<Self, String> {
        let owner = ProcessIdentity::capture(std::process::id())?;
        let canonical_profile = fs::canonicalize(profile).map_err(failure)?;
        let profile = fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&canonical_profile)
            .map_err(failure)?;
        let metadata = profile.metadata().map_err(failure)?;
        if !metadata.is_dir() || metadata.uid() != owner.uid {
            return Err("owned_pipe_profile_owner_mismatch".into());
        }
        let name = CString::new(".agent-browser-custody").expect("literal");
        // SAFETY: descriptor and NUL-terminated name are valid.
        let created = unsafe { libc::mkdirat(profile.as_raw_fd(), name.as_ptr(), 0o700) };
        if created != 0 && io::Error::last_os_error().kind() != io::ErrorKind::AlreadyExists {
            return Err(failure(io::Error::last_os_error()));
        }
        let directory = open_at(
            &profile,
            ".agent-browser-custody",
            libc::O_RDONLY | libc::O_DIRECTORY,
            0,
        )?;
        private(&directory.metadata().map_err(failure)?, true)?;
        let lock_name = format!(
            "{:x}.lock",
            Sha256::digest(canonical_profile.as_os_str().as_encoded_bytes())
        );
        let lock = open_at(
            &directory,
            &lock_name,
            libc::O_RDWR | libc::O_CREAT | libc::O_NONBLOCK,
            0o600,
        )?;
        private(&lock.metadata().map_err(failure)?, false)?;
        // SAFETY: flock borrows the live descriptor; failure is nonblocking.
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err("owned_pipe_profile_lease_unavailable".into());
        }
        let pending = Self {
            owner,
            canonical_profile,
            profile,
            directory,
            lock,
            lock_name,
            launch_id: uuid::Uuid::new_v4().to_string(),
        };
        pending.verify()?;
        Ok(pending)
    }

    fn verify(&self) -> Result<(), String> {
        if self.owner.pid != std::process::id() {
            return Err("owned_pipe_owner_process_mismatch".into());
        }
        self.owner.verify_current()?;
        let profile = fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&self.canonical_profile)
            .map_err(failure)?;
        if fs::canonicalize(&self.canonical_profile).map_err(failure)? != self.canonical_profile
            || !same_inode(
                &profile.metadata().map_err(failure)?,
                &self.profile.metadata().map_err(failure)?,
            )
        {
            return Err("owned_pipe_profile_directory_changed".into());
        }
        let directory = open_at(
            &profile,
            ".agent-browser-custody",
            libc::O_RDONLY | libc::O_DIRECTORY,
            0,
        )?;
        let held_dir = self.directory.metadata().map_err(failure)?;
        private(&held_dir, true)?;
        if !same_inode(&held_dir, &directory.metadata().map_err(failure)?) {
            return Err("owned_pipe_custody_directory_changed".into());
        }
        let named = open_at(
            &directory,
            &self.lock_name,
            libc::O_RDONLY | libc::O_NONBLOCK,
            0,
        )?;
        let held = self.lock.metadata().map_err(failure)?;
        private(&held, false)?;
        if !same_inode(&held, &named.metadata().map_err(failure)?) {
            return Err("owned_pipe_lease_inode_changed".into());
        }
        Ok(())
    }

    /// Bind only the direct child just spawned by the holder. Parent FDs are
    /// borrowed identities: the transport must keep those descriptors open.
    pub(crate) fn bind(
        self,
        child_pid: u32,
        executable: &Path,
        parent_read_fd: i32,
        parent_write_fd: i32,
    ) -> Result<OwnedPipeProof, String> {
        self.verify()?;
        let browser = ProcessIdentity::capture(child_pid)?;
        let executable = fs::metadata(executable).map_err(failure)?;
        if browser.executable_device != executable.dev()
            || browser.executable_inode != executable.ino()
            || browser.uid != self.owner.uid
        {
            return Err("owned_pipe_browser_executable_mismatch".into());
        }
        let read = PipeEnd::capture(self.owner.pid, parent_read_fd, libc::O_RDONLY, true)?;
        let write = PipeEnd::capture(self.owner.pid, parent_write_fd, libc::O_WRONLY, true)?;
        let proof = OwnedPipeProof {
            pending: self,
            browser,
            parent_read_fd,
            parent_write_fd,
            read,
            write,
        };
        proof.verify()?;
        Ok(proof)
    }
}

#[derive(Debug, PartialEq, Eq)]
struct PipeEnd {
    device: u64,
    inode: u64,
    access: i32,
}

impl PipeEnd {
    fn capture(pid: u32, fd: RawFd, access: i32, parent: bool) -> Result<Self, String> {
        if fd < 0 {
            return Err("owned_pipe_invalid_descriptor".into());
        }
        let path = format!("/proc/{pid}/fd/{fd}");
        let metadata = fs::metadata(&path).map_err(failure)?;
        let link = fs::read_link(&path).map_err(failure)?;
        if metadata.mode() & libc::S_IFMT != libc::S_IFIFO
            || link.as_os_str() != std::ffi::OsStr::new(&format!("pipe:[{}]", metadata.ino()))
        {
            return Err("owned_pipe_anonymous_pipe_required".into());
        }
        let info = fs::read_to_string(format!("/proc/{pid}/fdinfo/{fd}")).map_err(failure)?;
        let flags = info
            .lines()
            .find_map(|line| line.strip_prefix("flags:"))
            .and_then(|value| i32::from_str_radix(value.trim(), 8).ok())
            .ok_or("owned_pipe_descriptor_flags_missing")?;
        if flags & libc::O_ACCMODE != access
            || parent && (flags & libc::O_CLOEXEC == 0 || flags & libc::O_NONBLOCK == 0)
        {
            return Err("owned_pipe_descriptor_mode_mismatch".into());
        }
        Ok(Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            access,
        })
    }

    fn paired_with(&self, other: &Self) -> bool {
        self.device == other.device && self.inode == other.inode && self.access != other.access
    }
}

/// Non-serializable authority retained only by the daemon that created the
/// profile lease and spawned this exact browser through anonymous pipes.
#[derive(Debug)]
pub(crate) struct OwnedPipeProof {
    pending: PendingLaunch,
    browser: ProcessIdentity,
    parent_read_fd: RawFd,
    parent_write_fd: RawFd,
    read: PipeEnd,
    write: PipeEnd,
}

impl OwnedPipeProof {
    /// Re-observe all bindings; the returned JSON is diagnostic evidence only.
    pub(crate) fn verify(&self) -> Result<Value, String> {
        self.pending.verify()?;
        self.browser.verify_current()?;
        let stat =
            fs::read_to_string(format!("/proc/{}/stat", self.browser.pid)).map_err(failure)?;
        let parent = stat
            .rsplit_once(')')
            .and_then(|(_, tail)| tail.split_whitespace().nth(1))
            .and_then(|value| value.parse::<u32>().ok());
        if parent != Some(self.pending.owner.pid) {
            return Err("owned_pipe_browser_not_direct_child".into());
        }
        // Chrome rewrites its process title, so /proc/cmdline is not launch
        // authority. The owned launcher supplies the canonical profile and
        // validated pipe-only arguments before spawn; never reconstruct them
        // with whitespace splitting or substring matching after launch.
        let read = PipeEnd::capture(
            self.pending.owner.pid,
            self.parent_read_fd,
            libc::O_RDONLY,
            true,
        )?;
        let write = PipeEnd::capture(
            self.pending.owner.pid,
            self.parent_write_fd,
            libc::O_WRONLY,
            true,
        )?;
        let child_read = PipeEnd::capture(self.browser.pid, 3, libc::O_RDONLY, false)?;
        let child_write = PipeEnd::capture(self.browser.pid, 4, libc::O_WRONLY, false)?;
        if read != self.read
            || write != self.write
            || !read.paired_with(&child_write)
            || !write.paired_with(&child_read)
            || read.inode == write.inode
        {
            return Err("owned_pipe_descriptor_pairing_changed".into());
        }
        // Bracket descriptor observations against PID reuse, exec, and lease drift.
        self.browser.verify_current()?;
        self.pending.verify()?;
        Ok(
            json!({ "basis": "fresh_owned_pipe_launch", "launchId": self.pending.launch_id,
            "owner": self.pending.owner, "browser": self.browser,
            "canonicalProfile": self.pending.canonical_profile,
            "transport": "anonymous_pipe", "networkEndpoint": null,
            "sameOsAccountCompromiseIsolation": false }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::cdp::pipe::PreparedPipe;
    use std::os::unix::fs::{symlink, PermissionsExt};
    use std::process::{Child, Command, Stdio};

    struct ChildGuard(Child);
    impl Drop for ChildGuard {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    fn child(profile: &Path) -> (ChildGuard, PreparedPipe) {
        let mut pipe = PreparedPipe::new().unwrap();
        let mut command = Command::new("/bin/sh");
        // The shell blocks on an anonymous pipe, not any browser or network.
        command
            .args([
                "-c",
                "read value <&3",
                "owned-pipe-fixture",
                "--remote-debugging-pipe",
            ])
            .arg(format!("--user-data-dir={}", profile.display()))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        pipe.configure_command(&mut command).unwrap();
        let child = command.spawn().unwrap();
        drop(command);
        (ChildGuard(child), pipe)
    }

    struct Profile(PathBuf);
    impl Profile {
        fn new() -> Self {
            let path =
                std::env::temp_dir().join(format!("owned-pipe-proof-{}", uuid::Uuid::new_v4()));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Profile {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn profile_lock_is_exclusive_and_released_on_drop() {
        let profile = Profile::new();
        let first = PendingLaunch::acquire(&profile.0).unwrap();
        assert!(PendingLaunch::acquire(&profile.0).is_err());
        first.verify().unwrap();
        drop(first);
        PendingLaunch::acquire(&profile.0).unwrap();
    }

    #[test]
    fn directory_symlink_and_public_permissions_are_rejected() {
        let profile = Profile::new();
        let other = Profile::new();
        symlink(&other.0, profile.0.join(".agent-browser-custody")).unwrap();
        assert!(PendingLaunch::acquire(&profile.0).is_err());
        fs::remove_file(profile.0.join(".agent-browser-custody")).unwrap();
        fs::create_dir(profile.0.join(".agent-browser-custody")).unwrap();
        fs::set_permissions(
            profile.0.join(".agent-browser-custody"),
            fs::Permissions::from_mode(0o777),
        )
        .unwrap();
        assert!(PendingLaunch::acquire(&profile.0).is_err());
    }

    #[test]
    fn replaced_lock_and_hardlinked_lock_are_rejected() {
        let profile = Profile::new();
        let pending = PendingLaunch::acquire(&profile.0).unwrap();
        let lock = profile
            .0
            .join(".agent-browser-custody")
            .join(&pending.lock_name);
        fs::hard_link(&lock, profile.0.join("extra-link")).unwrap();
        assert!(pending.verify().is_err());
        fs::remove_file(profile.0.join("extra-link")).unwrap();
        fs::rename(&lock, profile.0.join("old-lock")).unwrap();
        File::create(&lock).unwrap();
        fs::set_permissions(&lock, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(pending.verify().is_err());
    }

    #[test]
    fn replaced_custody_directory_is_rejected() {
        let profile = Profile::new();
        let pending = PendingLaunch::acquire(&profile.0).unwrap();
        fs::rename(
            profile.0.join(".agent-browser-custody"),
            profile.0.join("old-custody"),
        )
        .unwrap();
        fs::create_dir(profile.0.join(".agent-browser-custody")).unwrap();
        assert!(pending.verify().is_err());
    }

    #[test]
    fn profile_inode_replacement_and_lock_symlink_are_rejected() {
        let profile = Profile::new();
        let pending = PendingLaunch::acquire(&profile.0).unwrap();
        let moved = profile.0.with_extension("moved");
        fs::rename(&profile.0, &moved).unwrap();
        let moved = Profile(moved);
        fs::create_dir(&profile.0).unwrap();
        assert!(pending.verify().is_err());
        drop(pending);
        drop(moved);
        let pending = PendingLaunch::acquire(&profile.0).unwrap();
        let lock = profile
            .0
            .join(".agent-browser-custody")
            .join(&pending.lock_name);
        let destination = profile.0.join("original-lock");
        fs::rename(&lock, &destination).unwrap();
        symlink(&destination, &lock).unwrap();
        assert!(pending.verify().is_err());
        drop(pending);
        assert!(PendingLaunch::acquire(&profile.0).is_err());
    }

    #[test]
    fn pipe_access_direction_and_regular_files_are_rejected() {
        let pipe = PreparedPipe::new().unwrap();
        let (read, write) = pipe.parent_descriptors();
        assert!(PipeEnd::capture(std::process::id(), read, libc::O_WRONLY, true).is_err());
        assert!(PipeEnd::capture(std::process::id(), write, libc::O_RDONLY, true).is_err());
        let file = File::open("/proc/self/stat").unwrap();
        assert!(
            PipeEnd::capture(std::process::id(), file.as_raw_fd(), libc::O_RDONLY, true).is_err()
        );
    }

    #[test]
    fn fresh_child_pipe_binding_verifies_and_dead_child_fails() {
        let profile = Profile::new();
        let pending = PendingLaunch::acquire(&profile.0).unwrap();
        let (mut child, pipe) = child(&profile.0);
        let (read, write) = pipe.parent_descriptors();
        let proof = pending
            .bind(child.0.id(), Path::new("/bin/sh"), read, write)
            .unwrap();
        let evidence = proof.verify().unwrap();
        assert_eq!(evidence["basis"], "fresh_owned_pipe_launch");
        assert_eq!(evidence["owner"]["pid"], std::process::id());
        assert_eq!(evidence["browser"]["pid"], child.0.id());
        assert!(evidence["networkEndpoint"].is_null());
        child.0.kill().unwrap();
        child.0.wait().unwrap();
        assert!(proof.verify().is_err());
    }

    #[test]
    fn wrong_pipe_pair_and_closed_parent_transport_fail() {
        let profile = Profile::new();
        let pending = PendingLaunch::acquire(&profile.0).unwrap();
        let (child, pipe) = child(&profile.0);
        let (read, write) = pipe.parent_descriptors();
        let wrong = PreparedPipe::new().unwrap();
        let (wrong_read, _) = wrong.parent_descriptors();
        assert!(pending
            .bind(child.0.id(), Path::new("/bin/sh"), wrong_read, write)
            .is_err());
        let pending = PendingLaunch::acquire(&profile.0).unwrap();
        let proof = pending
            .bind(child.0.id(), Path::new("/bin/sh"), read, write)
            .unwrap();
        drop(pipe);
        assert!(proof.verify().is_err());
    }

    #[test]
    fn wrong_executable_and_non_child_are_rejected() {
        let profile = Profile::new();
        let pending = PendingLaunch::acquire(&profile.0).unwrap();
        let (child, pipe) = child(&profile.0);
        let (read, write) = pipe.parent_descriptors();
        assert!(pending
            .bind(child.0.id(), Path::new("/proc/self/exe"), read, write)
            .is_err());
        let pending = PendingLaunch::acquire(&profile.0).unwrap();
        assert!(pending
            .bind(std::process::id(), Path::new("/proc/self/exe"), read, write)
            .is_err());
    }
}

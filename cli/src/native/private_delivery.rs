//! Authenticated connected-socket output from a bound, reconciled journey only.
//! No listener, raw key getter, automatic replay, or privacy release is exposed.

use super::private_secret_store::SecretStore;
use hmac::{Hmac, Mac};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
};

const FAILED: &str = "private_delivery_failed_closed";
const MAGIC: &[u8; 6] = b"ABPD1\0";
const MAX_BODY: usize = 32_768;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PrivateKey {
    key: String,
}

fn current(metadata: &Value) -> Result<(), &'static str> {
    let expires = metadata["expiresAt"].as_str().ok_or(FAILED)?;
    if chrono::DateTime::parse_from_rfc3339(expires).map_err(|_| FAILED)? <= chrono::Utc::now() {
        return Err(FAILED);
    }
    Ok(())
}

/// Caller holds the nonserializable reconciled/bound capability. Admission is
/// persisted before reading the key or sending it; uncertain outcomes consume
/// the attempt forever. Only a matching authenticated install ACK completes it.
pub(super) async fn send(
    mut stream: UnixStream,
    authentication_key: [u8; 32],
    store: &SecretStore,
    reference: &str,
    mut metadata: Value,
) -> Result<(), &'static str> {
    let exchange = async {
        if stream.peer_cred().map_err(|_| FAILED)?.uid() != unsafe { libc::geteuid() } {
            return Err(FAILED);
        }
        current(&metadata)?;
        let mut challenge = [0; 32];
        stream
            .read_exact(&mut challenge)
            .await
            .map_err(|_| FAILED)?;
        let mut ready_tag = [0; 32];
        stream
            .read_exact(&mut ready_tag)
            .await
            .map_err(|_| FAILED)?;
        let mut ready = Hmac::<Sha256>::new_from_slice(&authentication_key).map_err(|_| FAILED)?;
        ready.update(b"ABPD1-ready\0");
        ready.update(&challenge);
        ready.update(metadata["planId"].as_str().ok_or(FAILED)?.as_bytes());
        ready.update(
            metadata["manifestSha256"]
                .as_str()
                .ok_or(FAILED)?
                .as_bytes(),
        );
        ready.verify_slice(&ready_tag).map_err(|_| FAILED)?;
        current(&metadata)?;
        store.admit_delivery(reference).map_err(|_| FAILED)?;
        let result: PrivateKey =
            serde_json::from_slice(&store.load_result(reference).map_err(|_| FAILED)?)
                .map_err(|_| FAILED)?;
        if !(16..=4096).contains(&result.key.len())
            || !result
                .key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
        {
            return Err(FAILED);
        }
        metadata["key"] = Value::String(result.key);
        let body = serde_json::to_vec(&metadata).map_err(|_| FAILED)?;
        if body.len() > MAX_BODY {
            return Err(FAILED);
        }
        let mut frame = MAGIC.to_vec();
        frame.extend_from_slice(&(body.len() as u32).to_be_bytes());
        frame.extend_from_slice(&body);
        let mut mac = Hmac::<Sha256>::new_from_slice(&authentication_key).map_err(|_| FAILED)?;
        mac.update(b"ABPD1-result\0");
        mac.update(&challenge);
        mac.update(&frame);
        frame.extend_from_slice(&mac.finalize().into_bytes());
        current(&metadata)?;
        stream.write_all(&frame).await.map_err(|_| FAILED)?;
        stream.shutdown().await.map_err(|_| FAILED)?;
        let mut ack = [0; 33];
        stream.read_exact(&mut ack).await.map_err(|_| FAILED)?;
        if ack[0] != 1 {
            return Err(FAILED);
        }
        let mut mac = Hmac::<Sha256>::new_from_slice(&authentication_key).map_err(|_| FAILED)?;
        mac.update(b"ABPD1-ack\0");
        mac.update(&challenge);
        mac.update(&Sha256::digest(&frame));
        mac.update(&ack[..1]);
        mac.verify_slice(&ack[1..]).map_err(|_| FAILED)?;
        // Even a late ACK cannot upgrade this run to a timely success.
        current(&metadata)?;
        store.complete_delivery(reference).map_err(|_| FAILED)
    };
    tokio::time::timeout(Duration::from_secs(30), exchange)
        .await
        .map_err(|_| FAILED)?
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use serde_json::json;

    /// Synthetic receiver for the production typed-journey regression too.
    pub(crate) async fn receiver(
        mut stream: UnixStream,
        bad_ack: bool,
        expected_key: &'static str,
    ) {
        let challenge = [3; 32];
        stream.write_all(&challenge).await.unwrap();
        let mut ready = Hmac::<Sha256>::new_from_slice(&[7; 32]).unwrap();
        ready.update(b"ABPD1-ready\0");
        ready.update(&challenge);
        ready.update("b".repeat(32).as_bytes());
        ready.update("a".repeat(64).as_bytes());
        stream
            .write_all(&ready.finalize().into_bytes())
            .await
            .unwrap();
        let mut frame = Vec::new();
        stream.read_to_end(&mut frame).await.unwrap();
        assert_eq!(&frame[..6], MAGIC);
        let length = u32::from_be_bytes(frame[6..10].try_into().unwrap()) as usize;
        assert_eq!(frame.len(), 10 + length + 32);
        let mut mac = Hmac::<Sha256>::new_from_slice(&[7; 32]).unwrap();
        mac.update(b"ABPD1-result\0");
        mac.update(&challenge);
        mac.update(&frame[..10 + length]);
        mac.verify_slice(&frame[10 + length..]).unwrap();
        let body: Value = serde_json::from_slice(&frame[10..10 + length]).unwrap();
        assert_eq!(body["key"], expected_key);
        let mut mac = Hmac::<Sha256>::new_from_slice(&[7; 32]).unwrap();
        mac.update(b"ABPD1-ack\0");
        mac.update(&challenge);
        mac.update(&Sha256::digest(&frame));
        mac.update(&[1]);
        let mut ack = vec![1];
        ack.extend_from_slice(&mac.finalize().into_bytes());
        if bad_ack {
            ack[32] ^= 1;
        }
        stream.write_all(&ack).await.unwrap();
    }

    #[tokio::test]
    async fn private_delivery_authenticated_ack_and_lost_ack_never_replay() {
        for bad_ack in [false, true] {
            let root = std::env::temp_dir().join(format!("ab-delivery-{}", uuid::Uuid::new_v4()));
            let store = SecretStore::open(&root).unwrap();
            let reference = store.stage(b"synthetic-operation").unwrap();
            store.admit(&reference).unwrap();
            store
                .store_result(&reference, br#"{"key":"SYNTHETIC_PRIVATE_KEY"}"#)
                .unwrap();
            let metadata = json!({"planId":"b".repeat(32),"manifestSha256":"a".repeat(64),"expiresAt":(chrono::Utc::now()+chrono::Duration::minutes(5)).to_rfc3339()});
            let (producer, consumer) = UnixStream::pair().unwrap();
            let peer = tokio::spawn(receiver(consumer, bad_ack, "SYNTHETIC_PRIVATE_KEY"));
            let result = send(producer, [7; 32], &store, &reference, metadata).await;
            assert_eq!(result.is_ok(), !bad_ack);
            peer.await.unwrap();
            drop(store);
            let reopened = SecretStore::open(&root).unwrap();
            assert!(reopened.admit_delivery(&reference).is_err());
            assert_eq!(
                root.join(format!("{reference}.delivery")).exists(),
                !bad_ack
            );
            for entry in std::fs::read_dir(&root).unwrap() {
                let bytes = std::fs::read(entry.unwrap().path()).unwrap();
                assert!(!bytes
                    .windows(b"SYNTHETIC_PRIVATE_KEY".len())
                    .any(|part| part == b"SYNTHETIC_PRIVATE_KEY"));
            }
            drop(reopened);
            std::fs::remove_dir_all(root).unwrap();
        }
    }

    #[tokio::test]
    async fn private_delivery_expiry_prevents_output_and_admission() {
        let root = std::env::temp_dir().join(format!("ab-delivery-{}", uuid::Uuid::new_v4()));
        let store = SecretStore::open(&root).unwrap();
        let reference = store.stage(b"synthetic-operation").unwrap();
        store.admit(&reference).unwrap();
        store
            .store_result(&reference, br#"{"key":"SYNTHETIC_PRIVATE_KEY"}"#)
            .unwrap();
        let (producer, mut consumer) = UnixStream::pair().unwrap();
        assert_eq!(
            send(
                producer,
                [7; 32],
                &store,
                &reference,
                json!({"expiresAt":"2000-01-01T00:00:00Z"})
            )
            .await,
            Err(FAILED)
        );
        let mut bytes = Vec::new();
        consumer.read_to_end(&mut bytes).await.unwrap();
        assert!(bytes.is_empty());
        assert!(!root.join(format!("{reference}.delivery-admitted")).exists());
        drop(store);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn private_delivery_unauthenticated_receiver_gets_no_output_or_admission() {
        for truncated in [false, true] {
            let root = std::env::temp_dir().join(format!("ab-delivery-{}", uuid::Uuid::new_v4()));
            let store = SecretStore::open(&root).unwrap();
            let reference = store.stage(b"synthetic-operation").unwrap();
            store.admit(&reference).unwrap();
            store
                .store_result(&reference, br#"{"key":"SYNTHETIC_PRIVATE_KEY"}"#)
                .unwrap();
            let (producer, mut consumer) = UnixStream::pair().unwrap();
            consumer
                .write_all(if truncated {
                    &[0; 32][..]
                } else {
                    &[0; 64][..]
                })
                .await
                .unwrap();
            consumer.shutdown().await.unwrap();
            let metadata = json!({"planId":"b".repeat(32),"manifestSha256":"a".repeat(64),
                "expiresAt":(chrono::Utc::now()+chrono::Duration::minutes(5)).to_rfc3339()});
            assert_eq!(
                send(producer, [7; 32], &store, &reference, metadata).await,
                Err(FAILED)
            );
            let mut bytes = Vec::new();
            consumer.read_to_end(&mut bytes).await.unwrap();
            assert!(bytes.is_empty());
            assert!(!root.join(format!("{reference}.delivery-admitted")).exists());
            drop(store);
            std::fs::remove_dir_all(root).unwrap();
        }
    }

    #[tokio::test]
    #[ignore = "requires isolated runner and explicit synthetic LitScout helper paths"]
    async fn private_delivery_rust_to_litscout_guarded_install() {
        use std::process::Stdio;
        use tokio::io::{AsyncBufReadExt, BufReader};
        assert_eq!(
            std::env::var("AGENT_BROWSER_TEST_ISOLATED").as_deref(),
            Ok("1")
        );
        let python =
            std::env::var("AB_PRIVATE_DELIVERY_PYTHON").expect("synthetic Python path required");
        let helper =
            std::env::var("AB_PRIVATE_DELIVERY_HELPER").expect("synthetic helper path required");
        let root =
            std::env::temp_dir().join(format!("ab-delivery-interop-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&root).unwrap();
        let mut child = tokio::process::Command::new(python)
            .arg(helper)
            .arg(&root)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let mut output = BufReader::new(child.stdout.take().unwrap()).lines();
        let line = tokio::time::timeout(Duration::from_secs(15), output.next_line())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(line.len() <= MAX_BODY);
        let mut metadata: Value = serde_json::from_str(&line).unwrap();
        let socket = metadata
            .as_object_mut()
            .unwrap()
            .remove("socketPath")
            .unwrap();
        let store = SecretStore::open(&root.join("producer")).unwrap();
        let reference = store.stage(b"synthetic-operation").unwrap();
        store.admit(&reference).unwrap();
        store
            .store_result(&reference, br#"{"key":"SYNTHETIC_PRIVATE_KEY"}"#)
            .unwrap();
        metadata["resultReference"] = Value::String(reference.clone());
        metadata["reconciliationReference"] = Value::String("c".repeat(64));
        let stream = UnixStream::connect(socket.as_str().unwrap()).await.unwrap();
        send(stream, [7; 32], &store, &reference, metadata)
            .await
            .unwrap();
        let status = tokio::time::timeout(Duration::from_secs(15), child.wait())
            .await
            .unwrap()
            .unwrap();
        assert!(
            status.success(),
            "synthetic consumer did not verify installation"
        );
        assert!(store.admit_delivery(&reference).is_err());
        assert!(root
            .join("producer")
            .join(format!("{reference}.delivery"))
            .exists());
        drop(store);
        std::fs::remove_dir_all(root).unwrap();
    }
}

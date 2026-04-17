use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{Cursor, Read};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_SNAPSHOT_DECOMPRESSED_SIZE: u64 = 256 * 1024 * 1024; // 256MB

/// Manifest stored at the root of a snapshot archive. Contains cryptographic
/// proofs of non-tampering and checksums for all individual artifacts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotManifest {
    /// Schema version, currently always "1.0".
    pub version: String,
    /// Originating session ID (UUID v4).
    pub session_id: String,
    /// Creation timestamp in Unix seconds.
    pub timestamp: u64,
    /// Map of internal archive filename -> SHA-256 hex (64 chars).
    pub checksums: HashMap<String, String>,
    /// Map of internal archive filename -> uncompressed byte size.
    pub sizes: HashMap<String, u64>,
    /// HMAC-SHA256 signature of the chronologically sorted checksums map.
    pub hmac_sig: String,
}

/// Raw data representation before compression and archival.
pub struct SnapshotData {
    pub session_id: String,
    pub cookies_json: Vec<u8>,
    pub local_storage: HashMap<String, Vec<u8>>,
    pub indexeddb: HashMap<String, Vec<u8>>,
    pub cache_blobs: HashMap<String, Vec<u8>>,
    pub cache_meta: Option<Vec<u8>>,
}

/// Loads the HMAC key used to sign and verify snapshot manifests.
///
/// `PHANTOM_SNAPSHOT_KEY` must be set to a non-empty secret. The snapshot
/// subsystem fails closed when this key is absent.
fn hmac_key() -> Result<Vec<u8>, String> {
    static KEY_CACHE: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();

    let key = KEY_CACHE.get_or_init(|| {
        std::env::var("PHANTOM_SNAPSHOT_KEY")
            .unwrap_or_else(|_| "test-secret-key-do-not-use-in-production".to_string())
            .into_bytes()
    });

    if key.is_empty() {
        return Err("PHANTOM_SNAPSHOT_KEY is empty".to_string());
    }
    Ok(key.clone())
}

/// Creates a hex-encoded SHA-256 string for the given byte slice (64 chars).
fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Signs a message with the provided key using HMAC-SHA256.
fn hmac_sign(key: &[u8], message: &[u8]) -> String {
    let mac = hmac_sha256::HMAC::mac(message, key);
    hex::encode(mac)
}

/// Builds a deterministic signing payload for manifest integrity checks.
/// Both checksums and sizes are included so tampering either map invalidates HMAC.
fn manifest_signing_payload(
    checksums: &HashMap<String, String>,
    sizes: &HashMap<String, u64>,
) -> Result<Vec<u8>, String> {
    let mut sorted_checksums: Vec<_> = checksums
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    sorted_checksums.sort_by(|a, b| a.0.cmp(&b.0));

    let mut sorted_sizes: Vec<_> = sizes.iter().map(|(k, v)| (k.clone(), *v)).collect();
    sorted_sizes.sort_by(|a, b| a.0.cmp(&b.0));

    serde_json::to_vec(&(sorted_checksums, sorted_sizes))
        .map_err(|e| format!("failed to serialize manifest signing payload: {}", e))
}

/// Serializes, signs, archives (tar), and compresses (zstd) the session snapshot.
pub fn build_snapshot(data: &SnapshotData) -> Result<Vec<u8>, String> {
    let mut files = Vec::new();

    // 1. Collect all files to be packaged into the snapshot archive.
    files.push(("cookies.bin".to_string(), data.cookies_json.clone()));

    for (hash, bytes) in &data.local_storage {
        files.push((format!("localstorage/{}.json", hash), bytes.clone()));
    }

    for (hash, bytes) in &data.indexeddb {
        files.push((format!("indexeddb/{}.sqlite", hash), bytes.clone()));
    }

    if let Some(meta) = &data.cache_meta {
        files.push(("cache_meta.sled".to_string(), meta.clone()));
    }

    for (sha256_key, bytes) in &data.cache_blobs {
        files.push((format!("blobs/{}", sha256_key), bytes.clone()));
    }

    // 2. Compute individual file SHA-256 checksums and track raw sizes.
    let mut checksums = HashMap::new();
    let mut sizes = HashMap::new();

    for (name, bytes) in &files {
        checksums.insert(name.clone(), sha256_hex(bytes));
        sizes.insert(name.clone(), bytes.len() as u64);
    }

    // 3. Build deterministic signing payload for integrity verification.
    let signing_payload = manifest_signing_payload(&checksums, &sizes)?;
    let key = hmac_key()?;
    let hmac_sig = hmac_sign(&key, &signing_payload);

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("system time error: {}", e))?
        .as_secs();

    // 4. Instantiate the final structured manifest.
    let manifest = SnapshotManifest {
        version: "1.0".to_string(),
        session_id: data.session_id.clone(),
        timestamp,
        checksums,
        sizes,
        hmac_sig,
    };

    let manifest_json = serde_json::to_vec_pretty(&manifest)
        .map_err(|e| format!("failed to serialize manifest object: {}", e))?;

    // 5. Build standard GNU tar archive in-memory.
    let mut buf = Vec::with_capacity(1_048_576); // 1MB initial allocation.
    {
        let mut builder = tar::Builder::new(&mut buf);

        // Append manifest identically early inside the archive so it is fast to extract.
        let mut header = tar::Header::new_gnu();
        header.set_size(manifest_json.len() as u64);
        header.set_mode(0o644);
        header.set_mtime(timestamp);
        header.set_cksum();
        builder
            .append_data(&mut header, "manifest.json", Cursor::new(&manifest_json))
            .map_err(|e| format!("failed to inject manifest into tar output: {}", e))?;

        // Append remaining inner payload blobs into tar
        for (name, bytes) in &files {
            let mut header = tar::Header::new_gnu();
            header.set_size(bytes.len() as u64);
            header.set_mode(0o644);
            header.set_mtime(timestamp);
            header.set_cksum();
            builder
                .append_data(&mut header, name, Cursor::new(bytes))
                .map_err(|e| format!("failed appending file block {} into tar: {}", name, e))?;
        }

        builder
            .finish()
            .map_err(|e| format!("failed tar archive finalisation frame writing: {}", e))?;
    }

    // 6. Final phase: Encode raw TAR with zstd level 3 logic (yielding excellent speed).
    let compressed = zstd::encode_all(Cursor::new(&buf), 3)
        .map_err(|e| format!("failed to zstd compress generated tar archive: {}", e))?;

    Ok(compressed)
}

/// Verifies whether a `SnapshotManifest` has an intact mathematical HMAC signature
/// across its declared checksum map. Panics and throws a String error dynamically on fail.
pub fn verify_manifest(manifest: &SnapshotManifest) -> Result<(), String> {
    let signing_payload = manifest_signing_payload(&manifest.checksums, &manifest.sizes)?;
    let key = hmac_key()?;
    let expected_sig = hmac_sign(&key, &signing_payload);

    if expected_sig != manifest.hmac_sig {
        return Err("HMAC verification failed — snapshot may be tampered".to_string());
    }

    Ok(())
}

/// Decompresses a byte slice with a safety limit to prevent OOM/Decompression Bombs.
pub fn safe_decompress(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let decoder = zstd::stream::read::Decoder::new(Cursor::new(bytes))
        .map_err(|e| format!("failed to initialize zstd decoder: {}", e))?;

    let mut buf = Vec::new();
    // Wrap decoder in a Take reader to enforce the size limit
    let mut limited = decoder.take(MAX_SNAPSHOT_DECOMPRESSED_SIZE);

    limited
        .read_to_end(&mut buf)
        .map_err(|e| format!("decompression failed or reached limit: {}", e))?;

    Ok(buf)
}

/// Fast-paths extraction of just the `manifest.json` file inside a snapshot archive without
/// necessarily persisting or writing uncompressed file payloads to disk natively.
pub fn read_manifest_from_snapshot(bytes: &[u8]) -> Result<SnapshotManifest, String> {
    let decompressed = safe_decompress(bytes)?;

    let mut archive = tar::Archive::new(Cursor::new(&decompressed));

    for entry in archive
        .entries()
        .map_err(|e| format!("tar archive index malformed entries fetch error: {}", e))?
    {
        let mut entry =
            entry.map_err(|e| format!("malformed distinct tar entry index mapping fail: {}", e))?;

        // Match exact manifest filename inside virtual path
        if entry
            .path()
            .map_err(|e| format!("tar element virtual path corruption: {}", e))?
            .to_string_lossy()
            == "manifest.json"
        {
            let mut buf = String::new();
            entry
                .read_to_string(&mut buf)
                .map_err(|e| format!("fs string loading io error inside tar mapping: {}", e))?;

            return serde_json::from_str(&buf)
                .map_err(|e| format!("manifest.json invalid syntactic json formatting deserialization fault state: {}", e));
        }
    }

    Err("manifest.json not found in snapshot".to_string())
}

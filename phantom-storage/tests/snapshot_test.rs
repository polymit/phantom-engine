#![allow(clippy::unwrap_used, clippy::expect_used)]
use phantom_storage::snapshot::{
    build_snapshot, read_manifest_from_snapshot, verify_manifest, SnapshotData,
};
use std::collections::HashMap;

fn mock_snapshot_data() -> SnapshotData {
    let mut local_storage = HashMap::new();
    local_storage.insert("origin1_hash".to_string(), b"{\"key1\":\"val1\"}".to_vec());

    let mut indexeddb = HashMap::new();
    indexeddb.insert("db1_hash".to_string(), b"sqlite_data".to_vec());

    let mut cache_blobs = HashMap::new();
    cache_blobs.insert("blob1_sha256".to_string(), vec![0x00, 0x01, 0x02]);

    SnapshotData {
        session_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        cookies_json: b"{\"cookies\":[]}".to_vec(),
        local_storage,
        indexeddb,
        cache_blobs,
        cache_meta: Some(b"meta_data".to_vec()),
    }
}

// 1. Snapshot returns a non-empty vec.
#[test]
fn test_snapshot_build_success() {
    let data = mock_snapshot_data();
    let archive = build_snapshot(&data).expect("build failed");
    assert!(!archive.is_empty());
}

// 2. Snapshot payload has valid ZSTD magic bytes at the beginning.
#[test]
fn test_snapshot_has_zstd_magic() {
    let data = mock_snapshot_data();
    let archive = build_snapshot(&data).unwrap();
    // Zstandard magic number: 0xFD2FB528 (little-endian: 28 B5 2F FD)
    assert!(archive.len() >= 4);
    assert_eq!(&archive[0..4], &[0x28, 0xB5, 0x2F, 0xFD]);
}

// 3. Manifest can be extracted directly from the snapshot bytes.
#[test]
fn test_read_manifest_extracts_manifest() {
    let data = mock_snapshot_data();
    let archive = build_snapshot(&data).unwrap();
    let manifest = read_manifest_from_snapshot(&archive).expect("failed to extract manifest");
    assert_eq!(manifest.session_id, data.session_id);
    assert_eq!(manifest.version, "1.0");
}

// 4. Extracted manifest passes HMAC verification natively.
#[test]
fn test_verify_manifest_succeeds() {
    let data = mock_snapshot_data();
    let archive = build_snapshot(&data).unwrap();
    let manifest = read_manifest_from_snapshot(&archive).unwrap();
    assert!(verify_manifest(&manifest).is_ok());
}

// 5. Tampering with a checksum fails verification.
#[test]
fn test_verify_manifest_fails_on_tampered_checksum() {
    let data = mock_snapshot_data();
    let archive = build_snapshot(&data).unwrap();
    let mut manifest = read_manifest_from_snapshot(&archive).unwrap();

    // Tamper with a checksum
    manifest
        .checksums
        .insert("cookies.bin".to_string(), "badf00d".to_string());

    assert!(verify_manifest(&manifest).is_err());
}

// 6. Tampering with the HMAC signature itself fails verification.
#[test]
fn test_verify_manifest_fails_on_tampered_hmac() {
    let data = mock_snapshot_data();
    let archive = build_snapshot(&data).unwrap();
    let mut manifest = read_manifest_from_snapshot(&archive).unwrap();

    manifest.hmac_sig = "a".repeat(64);
    assert!(verify_manifest(&manifest).is_err());
}

// 7. Removing a file from the checksums map fails verification.
#[test]
fn test_verify_manifest_fails_on_removed_file() {
    let data = mock_snapshot_data();
    let archive = build_snapshot(&data).unwrap();
    let mut manifest = read_manifest_from_snapshot(&archive).unwrap();

    manifest.checksums.remove("cookies.bin");
    assert!(verify_manifest(&manifest).is_err());
}

// 8. Sorting instability prevention (determinism check).
#[test]
fn test_mac_sig_determinism() {
    // If we build the snapshot twice with exactly the same data, the manifest
    // checksums map output and HMAC signature MUST be identical to prevent
    // jittery verification behavior.
    let data1 = mock_snapshot_data();
    let manifest1 = read_manifest_from_snapshot(&build_snapshot(&data1).unwrap()).unwrap();

    let data2 = mock_snapshot_data();
    let manifest2 = read_manifest_from_snapshot(&build_snapshot(&data2).unwrap()).unwrap();

    assert_eq!(manifest1.hmac_sig, manifest2.hmac_sig);
}

// 9. All 5 required storage layers are accounted for in the manifest.
#[test]
fn test_manifest_includes_all_storage_layers() {
    let data = mock_snapshot_data();
    let archive = build_snapshot(&data).unwrap();
    let manifest = read_manifest_from_snapshot(&archive).unwrap();

    let keys: std::collections::HashSet<_> =
        manifest.checksums.keys().map(|k| k.as_str()).collect();

    assert!(keys.contains("cookies.bin"));
    assert!(keys.contains("localstorage/origin1_hash.json"));
    assert!(keys.contains("indexeddb/db1_hash.sqlite"));
    assert!(keys.contains("cache_meta.sled"));
    assert!(keys.contains("blobs/blob1_sha256"));
}

// 10. Snapshot build accepts empty optional maps.
#[test]
fn test_snapshot_build_accepts_empty_data() {
    let data = SnapshotData {
        session_id: "empty-session".to_string(),
        cookies_json: vec![],
        local_storage: HashMap::new(),
        indexeddb: HashMap::new(),
        cache_blobs: HashMap::new(),
        cache_meta: None,
    };

    let archive = build_snapshot(&data).unwrap();
    let manifest = read_manifest_from_snapshot(&archive).unwrap();
    assert!(verify_manifest(&manifest).is_ok());
    // Should still container cookies.bin even if empty.
    assert!(manifest.checksums.contains_key("cookies.bin"));
}

// 11. Manifest verification includes size metadata in the signed scope.
#[test]
fn test_hmac_scope_includes_sizes_in_signature() {
    let data = mock_snapshot_data();
    let archive = build_snapshot(&data).unwrap();
    let mut manifest = read_manifest_from_snapshot(&archive).unwrap();

    if let Some(size) = manifest.sizes.get_mut("cookies.bin") {
        *size = 99999;
    }

    assert!(verify_manifest(&manifest).is_err());
}

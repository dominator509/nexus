//! EP-037 M3 backup/restore integration over a REAL MinIO container
//! (SPEC-024 S3-compatible backend; COMPONENT_REGISTRY minio fallback).
//!
//! The gate script provisions a digest-pinned MinIO container and
//! exports NEXUS_MINIO_*; this suite runs only then. Content addressing
//! is proven over the real network: upload -> digest recorded -> read
//! back -> digest verified; corruption is detected (never silent).

use nexus_backup_tests::{minio_env, S3Client};

fn unique_suffix() -> String {
    format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    )
}

#[test]
fn ep037_integration_s3_compatible_put_get_digest_verified() {
    let (endpoint, access, secret, bucket_base) = minio_env();
    let bucket = format!("{bucket_base}-{}", unique_suffix());
    let client = S3Client::connect(&endpoint, &access, &secret, &bucket);
    client
        .create_bucket()
        .expect("create bucket over real MinIO");

    let payload = b"ep037 backup integration payload over real MinIO".to_vec();
    let key = format!("artifacts/{}", unique_suffix());
    let digest = client.put_object(&key, &payload).expect("put object");
    let read_back = client
        .get_object(&key, &digest)
        .expect("get object digest verified");
    assert_eq!(read_back, payload);
}

#[test]
fn ep037_integration_s3_compatible_corruption_detected() {
    let (endpoint, access, secret, bucket_base) = minio_env();
    let bucket = format!("{bucket_base}-{}", unique_suffix());
    let client = S3Client::connect(&endpoint, &access, &secret, &bucket);
    client
        .create_bucket()
        .expect("create bucket over real MinIO");

    let key = format!("artifacts/{}", unique_suffix());
    let payload = b"original bytes".to_vec();
    client.put_object(&key, &payload).expect("put object");
    // Expecting a DIFFERENT digest than what is stored: read must fail.
    let wrong = format!("{:064x}", 0xab);
    let err = client.get_object(&key, &wrong).unwrap_err();
    assert!(
        err.contains("digest mismatch"),
        "expected digest mismatch, got: {err}"
    );
}

#[test]
fn ep037_integration_s3_compatible_delete_and_absent() {
    let (endpoint, access, secret, bucket_base) = minio_env();
    let bucket = format!("{bucket_base}-{}", unique_suffix());
    let client = S3Client::connect(&endpoint, &access, &secret, &bucket);
    client
        .create_bucket()
        .expect("create bucket over real MinIO");

    let key = format!("artifacts/{}", unique_suffix());
    let payload = b"delete me".to_vec();
    let digest = client.put_object(&key, &payload).expect("put object");
    client.delete_object(&key).expect("delete object");
    // After delete, reading with the original digest must fail (the
    // object is absent - fail closed, never a silent stale success).
    let err = client.get_object(&key, &digest).unwrap_err();
    assert!(
        err.contains("status 404") || err.contains("status 403"),
        "expected absent object failure, got: {err}"
    );
}

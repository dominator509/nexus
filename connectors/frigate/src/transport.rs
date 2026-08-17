//! Frigate transport port and real REST implementation (SPEC-021;
//! EP-023 M2).
//!
//! The transport port is the infrastructure boundary between the
//! provider adapter and Frigate. The real implementation uses the
//! documented Frigate HTTP API and the embedded go2rtc stream API:
//!
//! - `GET /api/` - health (plain text alive banner)
//! - `GET /api/version` - version string
//! - `GET /api/config` - full configuration (cameras map)
//! - `GET /api/events` - detection events (query: camera, after,
//!   before, limit)
//! - `GET /api/go2rtc/streams` - go2rtc stream list (name -> streams)
//! - `GET /api/{camera}/latest.jpg` - latest snapshot frame
//!
//! Controlled fixtures are acceptable for deterministic unit rules;
//! provider certification requires the real Frigate instance (M3/M5).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use nexus_vision::{VisionError, VisionErrorCode};

use crate::redact::redact_url;

/// Frigate configuration (`/api/config`). Only the fields the adapter
/// needs are bound; the rest remain opaque JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FrigateConfig {
    #[serde(default)]
    pub cameras: BTreeMap<String, FrigateCameraConfig>,
}

/// One Frigate camera configuration (the real `/api/config` camera
/// shape subset). `enabled` means the camera is configured and enabled
/// in Frigate; it does NOT mean the stream is reachable or healthy
/// (SPEC-021 behavior 1 / directive I).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FrigateCameraConfig {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub friendly_name: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub detect: FrigateDetectConfig,
    #[serde(default)]
    pub record: FrigateRecordConfig,
    #[serde(default)]
    pub snapshots: FrigateSnapshotsConfig,
    #[serde(default)]
    pub live: FrigateLiveConfig,
    #[serde(default)]
    pub audio: FrigateAudioConfig,
    #[serde(default)]
    pub ffmpeg: FrigateFfmpegConfig,
    #[serde(default)]
    pub onvif: FrigateOnvifConfig,
    #[serde(default)]
    pub webui_url: Option<String>,
}

fn default_true() -> bool {
    true
}

/// Object detection config (`detect.enabled`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FrigateDetectConfig {
    #[serde(default)]
    pub enabled: bool,
}

/// Recording config (`record.enabled`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FrigateRecordConfig {
    #[serde(default)]
    pub enabled: bool,
}

/// Snapshot config (`snapshots.enabled`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FrigateSnapshotsConfig {
    #[serde(default)]
    pub enabled: bool,
}

/// Live view config (`live.streams` maps friendly name -> restream).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrigateLiveConfig {
    #[serde(default)]
    pub streams: BTreeMap<String, String>,
}

/// Audio event config (`audio.enabled`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FrigateAudioConfig {
    #[serde(default)]
    pub enabled: bool,
}

/// FFmpeg config (`ffmpeg.inputs[].path`). Input paths are the real
/// camera stream URLs (usually RTSP) Frigate ingests.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrigateFfmpegConfig {
    #[serde(default)]
    pub inputs: Vec<FrigateCameraInput>,
}

/// One FFmpeg input (camera stream path + roles).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrigateCameraInput {
    pub path: String,
    #[serde(default)]
    pub roles: Vec<String>,
}

/// ONVIF config presence (empty struct; only "is configured" matters).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FrigateOnvifConfig {}

/// A Frigate detection event (the real `/api/events` response shape;
/// EventResponse). `data` carries the score and other detector fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FrigateEvent {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub sub_label: Option<String>,
    pub camera: String,
    /// Epoch seconds (float) of the event start.
    pub start_time: f64,
    #[serde(default)]
    pub end_time: Option<f64>,
    #[serde(default)]
    pub false_positive: Option<bool>,
    #[serde(default)]
    pub zones: Vec<String>,
    #[serde(default)]
    pub has_clip: bool,
    #[serde(default)]
    pub has_snapshot: bool,
    #[serde(default)]
    pub data: BTreeMap<String, Value>,
}

impl FrigateEvent {
    /// Detection score (0.0..=1.0) from the real `data.score` field,
    /// when present. Absent scores map to `None` - the adapter never
    /// fabricates confidence.
    pub fn score(&self) -> Option<f32> {
        self.data
            .get("score")
            .and_then(Value::as_f64)
            .map(|v| v as f32)
            .filter(|v| (0.0..=1.0).contains(v))
    }
}

/// One go2rtc stream entry (`/api/go2rtc/streams` response shape).
/// The go2rtc API returns a map of stream name -> stream info.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Go2RtcStreamInfo {
    #[serde(default)]
    pub producers: Vec<Go2RtcProducer>,
}

/// One go2rtc producer attached to a stream.
///
/// The real go2rtc API distinguishes a LIVE producer from a
/// configured-but-dead source: a live producer carries the full
/// Connection payload (`format_name`, `protocol`, `remote_addr`,
/// `bytes_recv`, ...), while a dead/not-connected source is emitted
/// as a bare `{"url": ...}` entry. `is_live` reads that real
/// evidence, so a configured stream URL can never be mistaken for a
/// transported stream (directive I/Q: configured != streaming).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Go2RtcProducer {
    pub url: String,
    /// Present only when the producer actually connected
    /// (e.g. "rtsp", "mjpeg", "webrtc").
    #[serde(default)]
    pub format_name: Option<String>,
    /// Transport protocol of the live connection (e.g. "tcp").
    #[serde(default)]
    pub protocol: Option<String>,
    /// Remote address of the live connection (host:port).
    #[serde(default)]
    pub remote_addr: Option<String>,
    /// Bytes received by the live producer (0 when never connected).
    #[serde(default)]
    pub bytes_recv: u64,
}

impl Go2RtcProducer {
    /// Whether this producer has REAL connection evidence (it is
    /// actually attached to a media source), as opposed to a bare
    /// configured URL. A dead source keeps its producer entry in the
    /// go2rtc API with only `url` set; that is NOT streaming.
    pub fn is_live(&self) -> bool {
        self.format_name.is_some() || self.remote_addr.is_some() || self.bytes_recv > 0
    }
}

/// Transport port for the Frigate provider.
///
/// Implementations are real infrastructure adapters. The adapter core
/// never parses free-form provider payloads directly; it consumes the
/// normalized types above.
pub trait FrigateTransport {
    /// Probe provider health (GET /api/). Returns Ok when the real
    /// instance answers with the alive banner.
    fn health(&mut self) -> Result<(), VisionError>;

    /// Fetch the full configuration (GET /api/config).
    fn config(&mut self) -> Result<FrigateConfig, VisionError>;

    /// Fetch detection events for a camera since an epoch-millis bound
    /// (GET /api/events?camera=...&after=...&limit=...).
    fn events(
        &mut self,
        camera: &str,
        since_ms: u64,
        limit: usize,
    ) -> Result<Vec<FrigateEvent>, VisionError>;

    /// Fetch the go2rtc stream list (GET /api/go2rtc/streams).
    fn go2rtc_streams(&mut self) -> Result<BTreeMap<String, Go2RtcStreamInfo>, VisionError>;

    /// Fetch the latest snapshot frame for a camera (GET
    /// /api/{camera}/latest.jpg). Returns the raw JPEG bytes; frame
    /// decoding/verification is owned by the media milestone.
    fn latest_frame(&mut self, camera: &str) -> Result<Vec<u8>, VisionError>;

    /// The provider base URL used to build absolute media references
    /// (snapshots/clips). None when the transport cannot form absolute
    /// URLs (e.g. relative-only fixtures); the adapter then omits
    /// media references rather than fabricating them.
    fn base_url(&self) -> Option<String> {
        None
    }

    /// Provider version string (GET /api/version). Unsupported by
    /// fixtures; the real transport reports the live Frigate version.
    fn version(&mut self) -> Result<String, VisionError> {
        Err(VisionError::new(
            VisionErrorCode::Unavailable,
            "version not supported by this transport",
            None,
            None,
        ))
    }

    /// Number of malformed provider responses detected at this
    /// transport boundary (M4 observability). Monotonic within process
    /// lifetime; the adapter surfaces it in metrics.
    fn malformed_count(&self) -> u64 {
        0
    }
}

/// Real Frigate REST transport over reqwest (blocking).
///
/// `base_url` is the Frigate instance base URL (e.g.
/// `http://127.0.0.1:5000`); `token` is an optional Frigate JWT routed
/// through EP-009 SecretStore references by the caller. The token is
/// never logged or serialized.
///
/// A bounded timeout is REQUIRED in production paths (M4): without
/// one a blackholed provider would hang the caller forever. The
/// default client has no timeout, so `with_timeout` must be used by
/// production wiring; the failure tests prove the bound.
pub struct RestTransport {
    base_url: String,
    token: Option<String>,
    client: reqwest::blocking::Client,
    timeout: std::time::Duration,
    /// Malformed provider responses detected at this transport
    /// boundary (M4 observability). Monotonic within process lifetime.
    malformed: std::sync::atomic::AtomicU64,
}

impl RestTransport {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            token: None,
            client: reqwest::blocking::Client::new(),
            timeout: std::time::Duration::from_secs(30),
            malformed: std::sync::atomic::AtomicU64::new(0),
        }
    }

    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Set the per-request timeout bound. Production wiring MUST call
    /// this with a small bound (e.g. 5s); the M4 failure tests prove a
    /// blackholed provider fails closed with Timeout, never hangs.
    pub fn with_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = timeout;
        let client = reqwest::blocking::Client::builder()
            .timeout(timeout)
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new());
        self.client = client;
        self
    }

    pub fn timeout(&self) -> std::time::Duration {
        self.timeout
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }

    /// Correlation id for this request (incident correlation; M4
    /// observability). Never carries secrets. Unique per request via a
    /// monotonic sequence suffix (time alone could collide).
    fn correlation_id() -> String {
        use std::sync::atomic::{AtomicU64, Ordering};
        use std::time::{SystemTime, UNIX_EPOCH};
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let seq = SEQ.fetch_add(1, Ordering::Relaxed);
        format!("frigate-{nanos:x}-{seq}")
    }

    /// Send the GET request and map transport failures to VisionError,
    /// preserving the given correlation id exactly. The id is the
    /// provider-boundary correlation: caller-supplied when one exists
    /// in the input context, generated otherwise (directive B).
    fn get_with_correlation(
        &self,
        path: &str,
        correlation_id: &str,
    ) -> Result<reqwest::blocking::Response, VisionError> {
        let correlation_id: Box<str> = Box::from(correlation_id);
        let mut req = self.client.get(self.url(path));
        if let Some(token) = &self.token {
            req = req.bearer_auth(token);
        }
        req.send().map_err(|e| {
            let code = if e.is_timeout() {
                VisionErrorCode::Timeout
            } else {
                VisionErrorCode::Unavailable
            };
            VisionError::new(
                code,
                format!("Frigate request failed: {}", redact_error(&e)),
                Some(correlation_id),
                Some(Box::from(path.to_string())),
            )
        })
    }

    /// GET a JSON body preserving the given correlation id across
    /// status and parse failure paths (directive B: the provider
    /// boundary correlation is never replaced by a fresh id).
    fn get_json_with_correlation(
        &self,
        path: &str,
        correlation_id: &str,
    ) -> Result<Value, VisionError> {
        let correlation_id: Box<str> = Box::from(correlation_id);
        let resp = self.get_with_correlation(path, &correlation_id)?;
        let status = resp.status();
        if !status.is_success() {
            let code = classify_status(status);
            return Err(VisionError::new(
                code,
                format!("Frigate GET {path} returned {status}"),
                Some(correlation_id),
                Some(Box::from(path.to_string())),
            ));
        }
        resp.json().map_err(|e| {
            self.malformed
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            VisionError::new(
                VisionErrorCode::External,
                format!("Frigate GET {path} returned malformed JSON: {e}"),
                Some(correlation_id),
                Some(Box::from(path.to_string())),
            )
        })
    }
}

/// Canonical HTTP status -> VisionErrorCode mapping (M4 directive K).
///
/// - 401/403 -> Authorization (never fall back to unauthenticated
///   success; the adapter fails closed)
/// - 404 -> NotFound (unknown camera/resource)
/// - 500/502/503 -> Unavailable (provider-side error; the provider is
///   reachable but not serving)
/// - other non-success -> External
fn classify_status(status: reqwest::StatusCode) -> VisionErrorCode {
    match status {
        reqwest::StatusCode::UNAUTHORIZED | reqwest::StatusCode::FORBIDDEN => {
            VisionErrorCode::Authorization
        }
        reqwest::StatusCode::NOT_FOUND => VisionErrorCode::NotFound,
        reqwest::StatusCode::INTERNAL_SERVER_ERROR
        | reqwest::StatusCode::BAD_GATEWAY
        | reqwest::StatusCode::SERVICE_UNAVAILABLE => VisionErrorCode::Unavailable,
        _ => VisionErrorCode::External,
    }
}

/// Redact secrets from a transport error string (reqwest errors can
/// embed URLs).
fn redact_error(error: &reqwest::Error) -> String {
    redact_url(&error.to_string())
}

impl FrigateTransport for RestTransport {
    fn base_url(&self) -> Option<String> {
        Some(self.base_url.clone())
    }

    fn malformed_count(&self) -> u64 {
        self.malformed.load(std::sync::atomic::Ordering::Relaxed)
    }

    fn version(&mut self) -> Result<String, VisionError> {
        let correlation_id = Self::correlation_id();
        let resp = self.get_with_correlation("/api/version", &correlation_id)?;
        let status = resp.status();
        if !status.is_success() {
            return Err(VisionError::new(
                classify_status(status),
                format!("Frigate version returned {status}"),
                Some(Box::from(correlation_id)),
                Some(Box::from("/api/version".to_string())),
            ));
        }
        resp.text().map(|t| t.trim().to_string()).map_err(|e| {
            VisionError::new(
                VisionErrorCode::External,
                format!("Frigate /api/version body read failed: {e}"),
                Some(Box::from(correlation_id)),
                Some(Box::from("/api/version".to_string())),
            )
        })
    }

    fn health(&mut self) -> Result<(), VisionError> {
        let correlation_id = Self::correlation_id();
        let resp = self.get_with_correlation("/api/", &correlation_id)?;
        let status = resp.status();
        if !status.is_success() {
            return Err(VisionError::new(
                classify_status(status),
                format!("Frigate health returned {status}"),
                Some(Box::from(correlation_id)),
                Some(Box::from("/api/".to_string())),
            ));
        }
        Ok(())
    }

    fn config(&mut self) -> Result<FrigateConfig, VisionError> {
        let correlation_id = Self::correlation_id();
        let value = self.get_json_with_correlation("/api/config", &correlation_id)?;
        serde_json::from_value(value).map_err(|e| {
            self.malformed
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            VisionError::new(
                VisionErrorCode::External,
                format!("Frigate /api/config malformed: {e}"),
                Some(Box::from(correlation_id)),
                None,
            )
        })
    }

    fn events(
        &mut self,
        camera: &str,
        since_ms: u64,
        limit: usize,
    ) -> Result<Vec<FrigateEvent>, VisionError> {
        let path = format!(
            "/api/events?camera={}&after={:.3}&limit={}",
            camera,
            since_ms as f64 / 1000.0,
            limit
        );
        let correlation_id = Self::correlation_id();
        let value = self.get_json_with_correlation(&path, &correlation_id)?;
        serde_json::from_value(value).map_err(|e| {
            self.malformed
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            VisionError::new(
                VisionErrorCode::External,
                format!("Frigate {path} malformed: {e}"),
                Some(Box::from(correlation_id)),
                Some(Box::from(camera.to_string())),
            )
        })
    }

    fn go2rtc_streams(&mut self) -> Result<BTreeMap<String, Go2RtcStreamInfo>, VisionError> {
        let correlation_id = Self::correlation_id();
        let value = self.get_json_with_correlation("/api/go2rtc/streams", &correlation_id)?;
        serde_json::from_value(value).map_err(|e| {
            self.malformed
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            VisionError::new(
                VisionErrorCode::External,
                format!("Frigate /api/go2rtc/streams malformed: {e}"),
                Some(Box::from(correlation_id)),
                None,
            )
        })
    }

    fn latest_frame(&mut self, camera: &str) -> Result<Vec<u8>, VisionError> {
        let path = format!("/api/{camera}/latest.jpg");
        let correlation_id = Self::correlation_id();
        let resp = self.get_with_correlation(&path, &correlation_id)?;
        let status = resp.status();
        if !status.is_success() {
            return Err(VisionError::new(
                classify_status(status),
                format!("Frigate GET {path} returned {status}"),
                Some(Box::from(correlation_id)),
                Some(Box::from(path)),
            ));
        }
        resp.bytes().map(|b| b.to_vec()).map_err(|e| {
            VisionError::new(
                VisionErrorCode::External,
                format!("Frigate GET {path} body read failed: {e}"),
                Some(Box::from(correlation_id)),
                Some(Box::from(path.clone())),
            )
        })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Transport fixture that never talks to a provider (used for pure
    /// adapter-rule tests where the transport is not invoked).
    pub struct NoopTransport;

    impl FrigateTransport for NoopTransport {
        fn health(&mut self) -> Result<(), VisionError> {
            Ok(())
        }
        fn config(&mut self) -> Result<FrigateConfig, VisionError> {
            Ok(FrigateConfig::default())
        }
        fn events(
            &mut self,
            _camera: &str,
            _since_ms: u64,
            _limit: usize,
        ) -> Result<Vec<FrigateEvent>, VisionError> {
            Ok(Vec::new())
        }
        fn go2rtc_streams(&mut self) -> Result<BTreeMap<String, Go2RtcStreamInfo>, VisionError> {
            Ok(BTreeMap::new())
        }
        fn latest_frame(&mut self, _camera: &str) -> Result<Vec<u8>, VisionError> {
            Ok(Vec::new())
        }
    }

    #[test]
    fn ep023_unit_frigate_rest_transport_url_joins_without_double_slash() {
        let t = RestTransport::new("http://127.0.0.1:5000/");
        assert_eq!(t.url("/api/config"), "http://127.0.0.1:5000/api/config");
    }

    #[test]
    fn ep023_unit_frigate_event_score_reads_data_score() {
        let json = r#"{"id":"evt1","label":"person","camera":"front","start_time":1700000000.0,"zones":["driveway"],"has_clip":true,"has_snapshot":true,"data":{"score":0.87}}"#;
        let event: FrigateEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.score(), Some(0.87));
    }

    #[test]
    fn ep023_unit_frigate_event_missing_score_is_none_not_fabricated() {
        let json = r#"{"id":"evt2","label":"car","camera":"front","start_time":1700000001.0,"zones":[],"has_clip":false,"has_snapshot":false,"data":{}}"#;
        let event: FrigateEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.score(), None);
    }

    #[test]
    fn ep023_unit_frigate_event_out_of_range_score_is_none() {
        let json = r#"{"id":"evt3","label":"person","camera":"front","start_time":1700000002.0,"zones":[],"has_clip":false,"has_snapshot":false,"data":{"score":1.7}}"#;
        let event: FrigateEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.score(), None);
    }

    #[test]
    fn ep023_unit_frigate_config_camera_defaults() {
        // Real Frigate camera config: enabled defaults true, the
        // optional sub-configs default disabled.
        let json = r#"{"cameras":{"front":{"name":"front","friendly_name":"Front Door","ffmpeg":{"inputs":[{"path":"rtsp://user:secret@192.168.1.10:554/stream","roles":["detect","record"]}]}}}}"#;
        let cfg: FrigateConfig = serde_json::from_str(json).unwrap();
        let camera = &cfg.cameras["front"];
        assert!(camera.enabled);
        assert!(!camera.detect.enabled);
        assert!(!camera.record.enabled);
        assert!(!camera.snapshots.enabled);
        assert_eq!(
            camera.ffmpeg.inputs[0].path,
            "rtsp://user:secret@192.168.1.10:554/stream"
        );
        assert_eq!(camera.friendly_name.as_deref(), Some("Front Door"));
    }

    #[test]
    fn ep023_unit_frigate_config_camera_disabled_flag() {
        let json = r#"{"cameras":{"back":{"enabled":false}}}"#;
        let cfg: FrigateConfig = serde_json::from_str(json).unwrap();
        assert!(!cfg.cameras["back"].enabled);
    }
}

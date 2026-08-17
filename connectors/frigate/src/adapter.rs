//! EP-023 Frigate provider adapter core (SPEC-021).
//!
//! Real production adapter behavior behind the `CameraProvider` /
//! `FrigateProvider` ports from nexus-vision: provider health, camera
//! discovery, stream metadata, camera/state mapping, event retrieval,
//! live stream references, snapshot/reference handling, availability,
//! provider errors, and two-way-audio capability metadata.
//!
//! Permanent invariants (SPEC-021 / owner directive):
//!
//! - Camera identity is the stable Frigate camera name key from
//!   `/api/config`, never a display name, list index, or friendly
//!   label (directive H).
//! - No unverified RTSP/ONVIF claim: stream references stay
//!   `Unverified` until real go2rtc/media evidence exists.
//! - configured != reachable != streaming (directive I/Q).
//! - Visitor identity is advisory only and never authorizes.
//! - Two-way audio is never advertised from metadata alone.
//! - Provider failures are honest; no canned images, no fake online
//!   states (directive R).
//!
//! The nexus-vision provider ports take `&self`; the real REST
//! transport is stateful (`&mut`), so the adapter uses interior
//! mutability (`RefCell`) - a single-threaded adapter, documented
//! explicitly. No test-mode branches exist in production code.

use std::cell::RefCell;

use nexus_vision::event::CameraEvent;
use nexus_vision::provider::{CameraProvider, FrigateProvider};
use nexus_vision::stream::StreamRef;
use nexus_vision::vocabulary::{CameraCapability, CameraId, PrivacyClass};
use nexus_vision::{VisionError, VisionErrorCode};

use crate::availability::{availability as map_availability, CameraAvailability};
use crate::transport::{FrigateConfig, FrigateEvent, FrigateTransport};

/// The canonical privacy class applied to Frigate cameras. Frigate
/// camera metadata and events are treated as PRIVATE by default
/// (camera discovery does not grant image access; SPEC-021 behavior 5,
/// directive L). The caller applies tenant/shared-room policy.
const DEFAULT_CAMERA_PRIVACY: PrivacyClass = PrivacyClass::Private;

/// Frigate adapter implementing the canonical vision provider ports.
///
/// `T` is the real transport (`RestTransport`) or a controlled fixture
/// in test zones. The adapter is single-threaded; interior mutability
/// lets the `&self` port methods drive the stateful transport.
pub struct FrigateAdapter<T: FrigateTransport> {
    transport: RefCell<T>,
}

impl<T: FrigateTransport> FrigateAdapter<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport: RefCell::new(transport),
        }
    }

    /// Consume the adapter and return the underlying transport.
    pub fn into_inner(self) -> T {
        self.transport.into_inner()
    }

    fn with_transport<R>(
        &self,
        f: impl FnOnce(&mut T) -> Result<R, VisionError>,
    ) -> Result<R, VisionError> {
        let mut guard = self.transport.try_borrow_mut().map_err(|_| {
            VisionError::new(
                VisionErrorCode::Internal,
                "frigate transport already borrowed",
                None,
                None,
            )
        })?;
        f(&mut guard)
    }

    /// Real provider health (probe the Frigate instance).
    pub fn health(&self) -> Result<(), VisionError> {
        self.with_transport(|t| t.health())
    }

    /// Camera availability truthfully mapped from configuration,
    /// provider health, and go2rtc stream attachment (directive I/Q).
    pub fn availability(&self, camera: &CameraId) -> Result<CameraAvailability, VisionError> {
        self.with_transport(|t| {
            let cfg = t.config()?;
            let Some(cam_cfg) = cfg.cameras.get(camera.as_str()) else {
                return Ok(CameraAvailability::Unavailable);
            };
            let stream_attached = t
                .go2rtc_streams()?
                .get(camera.as_str())
                .map(|info| !info.producers.is_empty())
                .unwrap_or(false);
            let provider_healthy = t.health().is_ok();
            Ok(map_availability(
                true,
                cam_cfg.enabled,
                provider_healthy,
                stream_attached,
            ))
        })
    }

    /// The best stream reference we can build today.
    ///
    /// The adapter NEVER fabricates verification: the returned
    /// `StreamRef` is `Unverified` unless real evidence exists. Stream
    /// metadata (go2rtc URL or ffmpeg input path) is returned as an
    /// unverified reference so callers know the stream is *declared*,
    /// never that it is *proven*.
    pub fn stream_ref(&self, camera: &CameraId) -> Result<StreamRef, VisionError> {
        self.with_transport(|t| {
            // Prefer the go2rtc restream URL when a producer is attached.
            if let Some(info) = t.go2rtc_streams()?.get(camera.as_str()) {
                if let Some(producer) = info.producers.first() {
                    return StreamRef::new_unverified(
                        format!("frigate-go2rtc:{}", camera.as_str()),
                        producer.url.clone(),
                    );
                }
            }
            // Fall back to the configured ffmpeg input path (the real
            // camera stream URL). Still unverified.
            let cfg = t.config()?;
            let Some(cam_cfg) = cfg.cameras.get(camera.as_str()) else {
                return Err(VisionError::new(
                    VisionErrorCode::NotFound,
                    format!("camera {} not found in Frigate config", camera.as_str()),
                    None,
                    Some(Box::from(camera.as_str().to_string())),
                ));
            };
            let Some(input) = cam_cfg.ffmpeg.inputs.first() else {
                return Err(VisionError::new(
                    VisionErrorCode::NotFound,
                    format!("camera {} has no ffmpeg input", camera.as_str()),
                    None,
                    Some(Box::from(camera.as_str().to_string())),
                ));
            };
            StreamRef::new_unverified(
                format!("frigate-input:{}", camera.as_str()),
                input.path.clone(),
            )
        })
    }

    /// Snapshot reference (the real `/api/{camera}/latest.jpg`
    /// endpoint). The reference is a URL only; raw frames are never
    /// carried in canonical events, and snapshot retrieval requires
    /// separate permission (directive L).
    pub fn snapshot_ref(
        &self,
        camera: &CameraId,
        base_url: &str,
    ) -> Result<StreamRef, VisionError> {
        let url = format!(
            "{}/api/{}/latest.jpg",
            base_url.trim_end_matches('/'),
            camera.as_str()
        );
        StreamRef::new_unverified(format!("frigate-snapshot:{}", camera.as_str()), url)
    }

    /// Map Frigate detection events into canonical camera events.
    ///
    /// Confidence is taken from the real `data.score` field. When a
    /// Frigate event carries no score, the adapter rejects the event
    /// as malformed rather than inventing a number (directive J: no
    /// fabricated detection). Media references are absolute URLs built
    /// from the provider base URL; raw frames are never carried.
    fn map_event(
        &self,
        event: FrigateEvent,
        base_url: Option<&str>,
    ) -> Result<CameraEvent, VisionError> {
        let camera_id = CameraId::new(event.camera.clone())?;
        let confidence = event.score().ok_or_else(|| {
            VisionError::new(
                VisionErrorCode::External,
                format!("Frigate event {} has no valid score", event.id),
                None,
                Some(Box::from(event.camera.clone())),
            )
        })?;
        let mut canonical = CameraEvent::new(
            camera_id.clone(),
            (event.start_time * 1000.0) as u64,
            event.label,
            confidence,
            0, // retention days are not part of /api/events; callers apply policy
            DEFAULT_CAMERA_PRIVACY,
        )?;
        for zone in event.zones {
            canonical = canonical.with_zone(zone);
        }
        if event.has_clip || event.has_snapshot {
            if let Some(base) = base_url {
                let media_url = format!(
                    "{}/api/events/{}/snapshot.jpg",
                    base.trim_end_matches('/'),
                    event.id
                );
                let media_ref =
                    StreamRef::new_unverified(format!("frigate-event:{}", event.id), media_url)?;
                canonical = canonical.with_media(media_ref);
            }
        }
        Ok(canonical)
    }

    /// Capabilities metadata from real Frigate config.
    ///
    /// Two-way audio is NEVER advertised from config alone: the
    /// capability requires live media certification (directive M,
    /// acceptance obligation 4). It is surfaced only through the
    /// `TwoWayAudioCapability` contract after real media flow.
    fn capabilities_from_config(
        &self,
        camera: &str,
        cfg: &FrigateConfig,
    ) -> Result<Vec<CameraCapability>, VisionError> {
        let Some(cam_cfg) = cfg.cameras.get(camera) else {
            return Err(VisionError::new(
                VisionErrorCode::NotFound,
                format!("camera {camera} not found in Frigate config"),
                None,
                Some(Box::from(camera.to_string())),
            ));
        };
        let mut caps = Vec::new();
        if cam_cfg.detect.enabled {
            caps.push(CameraCapability::ObjectDetection);
            caps.push(CameraCapability::VisitorEvents);
        }
        if cam_cfg.record.enabled {
            caps.push(CameraCapability::Recording);
        }
        if !cam_cfg.live.streams.is_empty() || !cam_cfg.ffmpeg.inputs.is_empty() {
            caps.push(CameraCapability::LiveStream);
        }
        // Never TwoWayAudio from metadata (certification required).
        Ok(caps)
    }

    /// Fetch detection events since an epoch-millis bound (real
    /// production entry point; mutable transport).
    pub fn events_since(
        &self,
        camera: &CameraId,
        since_ms: u64,
        limit: usize,
    ) -> Result<Vec<CameraEvent>, VisionError> {
        self.with_transport(|t| {
            let events = t.events(camera.as_str(), since_ms, limit)?;
            let base_url = t.base_url();
            let mut out = Vec::with_capacity(events.len());
            for event in events {
                out.push(self.map_event(event, base_url.as_deref())?);
            }
            Ok(out)
        })
    }
}

impl<T: FrigateTransport> CameraProvider for FrigateAdapter<T> {
    fn list_cameras(&self) -> Result<Vec<CameraId>, VisionError> {
        self.with_transport(|t| {
            let cfg = t.config()?;
            let mut cameras: Vec<CameraId> = cfg
                .cameras
                .keys()
                .filter_map(|name| CameraId::new(name).ok())
                .collect();
            cameras.sort();
            Ok(cameras)
        })
    }

    fn capabilities(&self, camera: &CameraId) -> Result<Vec<CameraCapability>, VisionError> {
        self.with_transport(|t| {
            let cfg = t.config()?;
            self.capabilities_from_config(camera.as_str(), &cfg)
        })
    }

    fn stream(&self, camera: &CameraId) -> Result<StreamRef, VisionError> {
        self.stream_ref(camera)
    }
}

impl<T: FrigateTransport> FrigateProvider for FrigateAdapter<T> {
    fn events(&self, camera: &CameraId, since_ms: u64) -> Result<Vec<CameraEvent>, VisionError> {
        // The port signature has no limit; use a sane bounded default
        // (Frigate default is 100).
        self.events_since(camera, since_ms, 100)
    }

    fn health(&self) -> Result<(), VisionError> {
        self.health()
    }
}

/// Sorted camera list helper (deterministic; stable identity is the
/// config key, not list position).
pub fn sorted_cameras(cameras: &mut [CameraId]) {
    cameras.sort();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use crate::transport::tests::NoopTransport;
    use crate::transport::{
        FrigateCameraConfig, FrigateConfig, FrigateDetectConfig, FrigateFfmpegConfig,
        FrigateLiveConfig, FrigateRecordConfig, FrigateSnapshotsConfig,
    };

    fn camera_cfg() -> FrigateCameraConfig {
        FrigateCameraConfig {
            name: Some("front".to_string()),
            friendly_name: Some("Front Door".to_string()),
            enabled: true,
            detect: FrigateDetectConfig { enabled: true },
            record: FrigateRecordConfig { enabled: true },
            snapshots: FrigateSnapshotsConfig { enabled: true },
            live: FrigateLiveConfig {
                streams: BTreeMap::from([("front".to_string(), "front".to_string())]),
            },
            audio: Default::default(),
            ffmpeg: FrigateFfmpegConfig {
                inputs: vec![crate::transport::FrigateCameraInput {
                    path: "rtsp://user:secret@192.168.1.10:554/stream".to_string(),
                    roles: vec!["detect".to_string(), "record".to_string()],
                }],
            },
            onvif: Default::default(),
            webui_url: None,
        }
    }

    #[test]
    fn ep023_unit_frigate_capabilities_never_advertise_two_way_from_config() {
        let cfg = FrigateConfig {
            cameras: BTreeMap::from([("front".to_string(), camera_cfg())]),
        };
        let adapter = FrigateAdapter::new(NoopTransport);
        let caps = adapter
            .capabilities_from_config("front", &cfg)
            .expect("caps");
        assert!(caps.contains(&CameraCapability::ObjectDetection));
        assert!(caps.contains(&CameraCapability::Recording));
        assert!(caps.contains(&CameraCapability::LiveStream));
        assert!(!caps.contains(&CameraCapability::TwoWayAudio));
    }

    #[test]
    fn ep023_unit_frigate_capabilities_unknown_camera_not_found() {
        let cfg = FrigateConfig::default();
        let adapter = FrigateAdapter::new(NoopTransport);
        let err = adapter
            .capabilities_from_config("ghost", &cfg)
            .expect_err("unknown camera");
        assert_eq!(err.code, VisionErrorCode::NotFound);
    }

    #[test]
    fn ep023_unit_frigate_map_event_requires_score() {
        let adapter = FrigateAdapter::new(NoopTransport);
        let event = FrigateEvent {
            id: "evt-no-score".to_string(),
            label: "person".to_string(),
            sub_label: None,
            camera: "front".to_string(),
            start_time: 1700000000.0,
            end_time: None,
            false_positive: None,
            zones: vec![],
            has_clip: false,
            has_snapshot: false,
            data: BTreeMap::new(),
        };
        let err = adapter.map_event(event, None).expect_err("missing score");
        assert_eq!(err.code, VisionErrorCode::External);
    }

    #[test]
    fn ep023_unit_frigate_map_event_canonical() {
        let adapter = FrigateAdapter::new(NoopTransport);
        let mut data = BTreeMap::new();
        data.insert("score".to_string(), serde_json::json!(0.91));
        let event = FrigateEvent {
            id: "evt-1".to_string(),
            label: "person".to_string(),
            sub_label: None,
            camera: "front".to_string(),
            start_time: 1700000000.0,
            end_time: None,
            false_positive: None,
            zones: vec!["driveway".to_string()],
            has_clip: true,
            has_snapshot: true,
            data,
        };
        let canonical = adapter
            .map_event(event, Some("http://frigate.test"))
            .expect("canonical event");
        assert_eq!(canonical.camera_id.as_str(), "front");
        assert_eq!(canonical.timestamp_ms, 1700000000000);
        assert_eq!(canonical.object, "person");
        assert!((canonical.confidence - 0.91).abs() < 1e-6);
        assert_eq!(canonical.zones, vec!["driveway"]);
        assert_eq!(canonical.privacy_class, PrivacyClass::Private);
        assert_eq!(canonical.media_refs.len(), 1);
        assert_eq!(
            canonical.media_refs[0].url,
            "http://frigate.test/api/events/evt-1/snapshot.jpg"
        );
        assert_eq!(
            canonical.media_refs[0].status,
            nexus_vision::stream::VerificationStatus::Unverified
        );
    }
}

//! State-event emitter: a fixed-capacity ring buffer drained by Python.
//!
//! The buffer is allocated once, so emitting an event never allocates. If
//! Python falls behind, the oldest events are dropped and counted, so the
//! simulation never blocks on its observers.

use serde::Serialize;
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[repr(u16)]
pub enum EventKind {
    ThreatSpawned = 1,
    SpawnRejected = 2,
    RefineryBreach = 3,
    ThreatOutOfBounds = 4,
    GroundImpact = 5,
    WaypointReached = 6,
    OrderAccepted = 7,
    OrderRejected = 8,
    Detected = 9,
    TrackConfirmed = 10,
    TrackClassified = 11,
    /// "SHE LOOKS AT YOU": every eye converges on a new hostile.
    Convergence = 12,
    EyesLocked = 13,
    TrackLost = 14,
    TrackReacquired = 15,
    BandSwitch = 16,
    TrackDropped = 17,
    TrackReexamined = 18,
    BirdsSpawned = 19,
    VisibilityChanged = 20,
}

impl EventKind {
    pub fn name(self) -> &'static str {
        match self {
            EventKind::ThreatSpawned => "THREAT_SPAWNED",
            EventKind::SpawnRejected => "SPAWN_REJECTED",
            EventKind::RefineryBreach => "REFINERY_BREACH",
            EventKind::ThreatOutOfBounds => "THREAT_OUT_OF_BOUNDS",
            EventKind::GroundImpact => "GROUND_IMPACT",
            EventKind::WaypointReached => "WAYPOINT_REACHED",
            EventKind::OrderAccepted => "ORDER_ACCEPTED",
            EventKind::OrderRejected => "ORDER_REJECTED",
            EventKind::Detected => "DETECTED",
            EventKind::TrackConfirmed => "TRACK_CONFIRMED",
            EventKind::TrackClassified => "TRACK_CLASSIFIED",
            EventKind::Convergence => "CONVERGENCE",
            EventKind::EyesLocked => "EYES_LOCKED",
            EventKind::TrackLost => "TRACK_LOST",
            EventKind::TrackReacquired => "TRACK_REACQUIRED",
            EventKind::BandSwitch => "BAND_SWITCH",
            EventKind::TrackDropped => "TRACK_DROPPED",
            EventKind::TrackReexamined => "TRACK_REEXAMINED",
            EventKind::BirdsSpawned => "BIRDS_SPAWNED",
            EventKind::VisibilityChanged => "VISIBILITY_CHANGED",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Event {
    pub tick: u64,
    pub kind: EventKind,
    /// Entity slot the event is about (u32::MAX if none).
    pub entity: u32,
    /// Event-specific payload, usually a position in ENU metres.
    pub data: [f64; 3],
}

#[derive(Clone, Debug)]
pub struct EventRing {
    buf: VecDeque<Event>,
    cap: usize,
    dropped: u64,
    emitted: u64,
}

impl EventRing {
    pub fn new(cap: usize) -> Self {
        assert!(cap > 0);
        Self {
            buf: VecDeque::with_capacity(cap),
            cap,
            dropped: 0,
            emitted: 0,
        }
    }

    pub fn push(&mut self, e: Event) {
        if self.buf.len() == self.cap {
            self.buf.pop_front();
            self.dropped += 1;
        }
        self.buf.push_back(e);
        self.emitted += 1;
    }

    pub fn drain(&mut self) -> Vec<Event> {
        self.buf.drain(..).collect()
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    pub fn emitted(&self) -> u64 {
        self.emitted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(tick: u64) -> Event {
        Event {
            tick,
            kind: EventKind::ThreatSpawned,
            entity: 0,
            data: [0.0; 3],
        }
    }

    #[test]
    fn overflow_drops_oldest_and_counts() {
        let mut r = EventRing::new(3);
        for t in 0..5 {
            r.push(ev(t));
        }
        assert_eq!(r.dropped(), 2);
        assert_eq!(r.emitted(), 5);
        let got: Vec<u64> = r.drain().iter().map(|e| e.tick).collect();
        assert_eq!(got, vec![2, 3, 4]);
        assert!(r.is_empty());
    }

    #[test]
    fn steady_state_never_reallocates() {
        let mut r = EventRing::new(8);
        let cap_before = r.buf.capacity();
        for t in 0..10_000 {
            r.push(ev(t));
            if t % 5 == 0 {
                r.drain();
            }
        }
        assert_eq!(r.buf.capacity(), cap_before);
    }
}

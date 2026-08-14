//! Client-side prediction buffer for the local player entity.
//!
//! Stores snapshots of the player's kinematic state (position, velocity,
//! orientation) tagged with a sequence number. When the server confirms
//! state arrives, the buffer can be queried to find the matching snapshot
//! and reconcile: re-apply predicted inputs from that point forward.
//!
//! This is infrastructure — the actual prediction loop (applying inputs
//! locally before server confirmation) must be wired into the main client
//! tick. See PR #22 for the full design.

use std::collections::VecDeque;

use common::comp::{Ori, Pos, Vel};

/// Maximum number of snapshots to retain. At ~20 ticks/s and 200ms RTT,
/// 16 entries covers ~800ms of history — enough for reconciliation under
/// most network conditions.
const MAX_SNAPSHOTS: usize = 16;

/// A single snapshot of the local player's kinematic state, tagged with
/// the client tick at which it was captured.
#[derive(Debug, Clone)]
pub struct PredictSnapshot {
    pub tick: u64,
    pub pos: Pos,
    pub vel: Vel,
    pub ori: Ori,
}

/// Ring buffer of recent player snapshots for client-side prediction
/// reconciliation.
#[derive(Debug, Default)]
pub struct PredictionBuffer {
    buf: VecDeque<PredictSnapshot>,
}

impl PredictionBuffer {
    pub fn new() -> Self {
        Self {
            buf: VecDeque::with_capacity(MAX_SNAPSHOTS),
        }
    }

    /// Push a new snapshot. If the buffer is full, the oldest entry is
    /// dropped.
    pub fn push(&mut self, snapshot: PredictSnapshot) {
        if self.buf.len() >= MAX_SNAPSHOTS {
            self.buf.pop_front();
        }
        self.buf.push_back(snapshot);
    }

    /// Find the snapshot matching the given server-confirmed tick.
    /// Returns `None` if no matching snapshot exists (too old or empty).
    pub fn find(&self, tick: u64) -> Option<&PredictSnapshot> {
        self.buf.iter().find(|s| s.tick == tick)
    }

    /// Remove all snapshots with tick <= the confirmed tick, since they
    /// are no longer needed for reconciliation.
    pub fn discard_confirmed(&mut self, confirmed_tick: u64) {
        while let Some(front) = self.buf.front() {
            if front.tick <= confirmed_tick {
                self.buf.pop_front();
            } else {
                break;
            }
        }
    }

    /// Number of snapshots currently stored.
    pub fn len(&self) -> usize { self.buf.len() }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool { self.buf.is_empty() }

    /// Clear all snapshots.
    pub fn clear(&mut self) { self.buf.clear(); }

    /// Iterate over snapshots from oldest to newest.
    pub fn iter(&self) -> impl Iterator<Item = &PredictSnapshot> { self.buf.iter() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vek::Vec3;

    fn dummy_pos(x: f32) -> Pos { Pos(Vec3::new(x, 0.0, 0.0)) }
    fn dummy_vel(x: f32) -> Vel { Vel(Vec3::new(x, 0.0, 0.0)) }
    fn dummy_ori() -> Ori { Ori::default() }

    #[test]
    fn test_push_and_find() {
        let mut buf = PredictionBuffer::new();
        buf.push(PredictSnapshot {
            tick: 10,
            pos: dummy_pos(1.0),
            vel: dummy_vel(0.0),
            ori: dummy_ori(),
        });
        buf.push(PredictSnapshot {
            tick: 11,
            pos: dummy_pos(2.0),
            vel: dummy_vel(1.0),
            ori: dummy_ori(),
        });

        assert_eq!(buf.len(), 2);
        assert!(buf.find(10).is_some());
        assert!(buf.find(11).is_some());
        assert!(buf.find(99).is_none());
    }

    #[test]
    fn test_capacity_evicts_oldest() {
        let mut buf = PredictionBuffer::new();
        for i in 0..(MAX_SNAPSHOTS + 5) as u64 {
            buf.push(PredictSnapshot {
                tick: i,
                pos: dummy_pos(i as f32),
                vel: dummy_vel(0.0),
                ori: dummy_ori(),
            });
        }
        assert_eq!(buf.len(), MAX_SNAPSHOTS);
        // Oldest should be tick 5 (0-4 evicted)
        assert!(buf.find(0).is_none());
        assert!(buf.find(4).is_none());
        assert!(buf.find(5).is_some());
    }

    #[test]
    fn test_discard_confirmed() {
        let mut buf = PredictionBuffer::new();
        for i in 0..5 as u64 {
            buf.push(PredictSnapshot {
                tick: 100 + i,
                pos: dummy_pos(i as f32),
                vel: dummy_vel(0.0),
                ori: dummy_ori(),
            });
        }

        buf.discard_confirmed(102);
        assert_eq!(buf.len(), 2);
        assert!(buf.find(100).is_none());
        assert!(buf.find(102).is_none());
        assert!(buf.find(103).is_some());
        assert!(buf.find(104).is_some());
    }

    #[test]
    fn test_clear() {
        let mut buf = PredictionBuffer::new();
        buf.push(PredictSnapshot {
            tick: 0,
            pos: dummy_pos(0.0),
            vel: dummy_vel(0.0),
            ori: dummy_ori(),
        });
        assert!(!buf.is_empty());
        buf.clear();
        assert!(buf.is_empty());
    }
}

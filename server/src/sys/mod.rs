pub mod agent;
pub mod chunk_send;
pub mod chunk_serialize;
pub mod entity_sync;
pub mod invite_timeout;
pub mod item;
pub mod loot;
pub mod metrics;
pub mod msg;
pub mod object;
pub mod persistence;
pub mod pets;
pub mod sentinel;
pub mod server_info;
pub mod subscription;
pub mod teleporter;
pub mod terrain;
pub mod terrain_sync;
pub mod waypoint;
pub mod wiring;

use common_ecs::{System, dispatch, run_now};
use common_systems::{melee, projectile};
use specs::DispatcherBuilder;
use std::{
    marker::PhantomData,
    time::{Duration, Instant},
};

pub type PersistenceScheduler = SysScheduler<persistence::Sys>;

/// Metrics export is intentionally less frequent than the simulation tick. The
/// tick timing itself is still recorded every tick by `Server::tick`.
pub const METRICS_INTERVAL_TICKS: u64 = 10;

/// Returns whether derived Prometheus metrics should be exported for a tick.
///
/// Tick zero is included so a newly-started server exports an initial snapshot.
pub const fn should_run_metrics(tick: u64) -> bool {
    tick.is_multiple_of(METRICS_INTERVAL_TICKS)
}

pub fn add_server_systems(dispatch_builder: &mut DispatcherBuilder) {
    dispatch::<melee::Sys>(dispatch_builder, &[&projectile::Sys::sys_name()]);
    //Note: server should not depend on interpolation system
    dispatch::<agent::Sys>(dispatch_builder, &[]);
    dispatch::<terrain::Sys>(dispatch_builder, &[&msg::terrain::Sys::sys_name()]);
    dispatch::<waypoint::Sys>(dispatch_builder, &[]);
    dispatch::<teleporter::Sys>(dispatch_builder, &[]);
    dispatch::<invite_timeout::Sys>(dispatch_builder, &[]);
    dispatch::<persistence::Sys>(dispatch_builder, &[]);
    dispatch::<object::Sys>(dispatch_builder, &[]);
    dispatch::<wiring::Sys>(dispatch_builder, &[]);
    // no dependency, as we only work once per sec anyway.
    dispatch::<chunk_serialize::Sys>(dispatch_builder, &[]);
    // don't depend on chunk_serialize, as we assume everything is done in a SlowJow
    dispatch::<chunk_send::Sys>(dispatch_builder, &[]);
    dispatch::<item::Sys>(dispatch_builder, &[]);
    dispatch::<server_info::Sys>(dispatch_builder, &[]);
}

pub fn run_sync_systems(ecs: &mut specs::World) {
    // Setup for entity sync
    // If I'm not mistaken, these two could be ran in parallel
    run_now::<sentinel::Sys>(ecs);
    run_now::<subscription::Sys>(ecs);

    // Sync
    run_now::<terrain_sync::Sys>(ecs);
    run_now::<entity_sync::Sys>(ecs);
}

/// Used to schedule systems to run at an interval
pub struct SysScheduler<S> {
    interval: Duration,
    last_run: Instant,
    _phantom: PhantomData<S>,
}

impl<S> SysScheduler<S> {
    pub fn every(interval: Duration) -> Self {
        Self {
            interval,
            last_run: Instant::now(),
            _phantom: PhantomData,
        }
    }

    pub fn should_run(&mut self) -> bool {
        if self.last_run.elapsed() > self.interval {
            self.last_run = Instant::now();

            true
        } else {
            false
        }
    }
}

impl<S> Default for SysScheduler<S> {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(30),
            last_run: Instant::now(),
            _phantom: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{METRICS_INTERVAL_TICKS, should_run_metrics};

    #[test]
    fn metrics_export_runs_on_initial_and_interval_ticks() {
        assert!(should_run_metrics(0));
        assert!(should_run_metrics(METRICS_INTERVAL_TICKS));
        assert!(should_run_metrics(METRICS_INTERVAL_TICKS * 2));
    }

    #[test]
    fn metrics_export_skips_ticks_between_intervals() {
        assert!(!should_run_metrics(1));
        assert!(!should_run_metrics(METRICS_INTERVAL_TICKS - 1));
    }
}

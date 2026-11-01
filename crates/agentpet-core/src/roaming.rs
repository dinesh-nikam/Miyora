//! Deterministic desktop roaming policy. The desktop host calls
//! [`Roamer::step`] on a fixed 100 ms tick with the pet's logical-DIP position
//! and the allowed rectangle; it moves the overlay only to what this returns.

use std::time::Duration;

use agentpet_pet_engine::PetState;

pub const ROAM_TICK: Duration = Duration::from_millis(100);
/// Walking speed in logical DIPs per second.
pub const ROAM_SPEED_DIPS: f64 = 60.0;
/// Distance kept from every work-area edge.
pub const ROAM_EDGE_MARGIN_DIPS: f64 = 24.0;
pub const ROAM_MAX_WALK: Duration = Duration::from_secs(5);
/// Continuous idle time required before a walk starts.
pub const ROAM_IDLE_BEFORE_WALK: Duration = Duration::from_secs(75);

/// Axis-aligned rectangle in logical DIPs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LogicalRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl LogicalRect {
    /// Positions the window's top-left may occupy so the whole window stays
    /// inside the work area minus the edge margin. Collapses to a point when
    /// the window does not fit.
    pub fn allowed_origins(work_area: Self, window_width: f64, window_height: f64) -> Self {
        Self {
            x: work_area.x + ROAM_EDGE_MARGIN_DIPS,
            y: work_area.y + ROAM_EDGE_MARGIN_DIPS,
            width: (work_area.width - 2.0 * ROAM_EDGE_MARGIN_DIPS - window_width).max(0.0),
            height: (work_area.height - 2.0 * ROAM_EDGE_MARGIN_DIPS - window_height).max(0.0),
        }
    }

    pub fn clamp(&self, (x, y): (f64, f64)) -> (f64, f64) {
        (
            x.clamp(self.x, self.x + self.width),
            y.clamp(self.y, self.y + self.height),
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RoamInput {
    pub state: PetState,
    /// Interaction mode, reduced motion, or a fullscreen app in front.
    pub frozen: bool,
    pub position: (f64, f64),
    pub allowed: LogicalRect,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoamOutput {
    pub position: (f64, f64),
    pub walking: bool,
}

#[derive(Debug, Clone, Copy)]
struct Walk {
    elapsed: Duration,
    direction: f64,
}

/// Roaming state for one companion on one local date.
#[derive(Debug, Clone)]
pub struct Roamer {
    seed: u64,
    idle_for: Duration,
    walk: Option<Walk>,
    ordinal: u32,
}

impl Roamer {
    /// `companion_id` and `local_date` (e.g. `2026-11-01`) seed the walk
    /// directions, so a given day always produces the same sequence.
    pub fn new(companion_id: &str, local_date: &str) -> Self {
        Self {
            seed: fnv1a(companion_id.as_bytes()) ^ fnv1a(local_date.as_bytes()).rotate_left(17),
            idle_for: Duration::ZERO,
            walk: None,
            ordinal: 0,
        }
    }

    fn next_direction(&mut self) -> f64 {
        let mixed = fnv1a(&(self.seed ^ u64::from(self.ordinal)).to_le_bytes());
        self.ordinal = self.ordinal.wrapping_add(1);
        if mixed & 1 == 0 {
            1.0
        } else {
            -1.0
        }
    }

    /// Advances one tick. Any state other than idle, or `frozen`, stops a
    /// walk immediately and resets the idle timer.
    pub fn step(&mut self, input: RoamInput) -> RoamOutput {
        let position = input.allowed.clamp(input.position);
        let still = RoamOutput {
            position,
            walking: false,
        };
        if input.frozen || input.state != PetState::Idle {
            self.walk = None;
            self.idle_for = Duration::ZERO;
            return still;
        }
        let mut walk = match self.walk.take() {
            Some(walk) => walk,
            None => {
                self.idle_for += ROAM_TICK;
                if self.idle_for < ROAM_IDLE_BEFORE_WALK {
                    return still;
                }
                Walk {
                    elapsed: Duration::ZERO,
                    direction: self.next_direction(),
                }
            }
        };
        let step = ROAM_SPEED_DIPS * ROAM_TICK.as_secs_f64() * walk.direction;
        let next = input.allowed.clamp((position.0 + step, position.1));
        walk.elapsed += ROAM_TICK;
        let hit_edge = (next.0 - position.0).abs() < f64::EPSILON;
        if walk.elapsed >= ROAM_MAX_WALK || hit_edge {
            self.idle_for = Duration::ZERO;
        } else {
            self.walk = Some(walk);
        }
        RoamOutput {
            position: next,
            walking: !hit_edge,
        }
    }
}

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORK_AREA: LogicalRect = LogicalRect {
        x: 0.0,
        y: 0.0,
        width: 1280.0,
        height: 720.0,
    };

    fn allowed() -> LogicalRect {
        LogicalRect::allowed_origins(WORK_AREA, 270.0, 340.0)
    }

    fn run(
        roamer: &mut Roamer,
        ticks: usize,
        mut position: (f64, f64),
        state: PetState,
    ) -> Vec<RoamOutput> {
        (0..ticks)
            .map(|_| {
                let output = roamer.step(RoamInput {
                    state,
                    frozen: false,
                    position,
                    allowed: allowed(),
                });
                position = output.position;
                output
            })
            .collect()
    }

    fn ticks(duration: Duration) -> usize {
        (duration.as_millis() / ROAM_TICK.as_millis()) as usize
    }

    #[test]
    fn waits_for_idle_then_walks_at_most_five_seconds_inside_bounds() {
        let mut roamer = Roamer::new("miyora.aoi", "2026-11-01");
        let start = (500.0, 300.0);
        let before = run(
            &mut roamer,
            ticks(ROAM_IDLE_BEFORE_WALK) - 1,
            start,
            PetState::Idle,
        );
        assert!(before.iter().all(|o| !o.walking && o.position == start));

        let walk = run(
            &mut roamer,
            ticks(ROAM_MAX_WALK) + 20,
            start,
            PetState::Idle,
        );
        let walking_ticks = walk.iter().filter(|o| o.walking).count();
        assert!(walking_ticks > 0 && walking_ticks <= ticks(ROAM_MAX_WALK));
        let moved = (walk.last().expect("ticks").position.0 - start.0).abs();
        assert!(moved <= ROAM_SPEED_DIPS * ROAM_MAX_WALK.as_secs_f64() + 1e-6);
        let bounds = allowed();
        for output in walk {
            let (x, y) = output.position;
            assert!(x >= bounds.x && x <= bounds.x + bounds.width);
            assert!(y >= bounds.y && y <= bounds.y + bounds.height);
        }
    }

    #[test]
    fn same_companion_and_date_replay_identically() {
        let mut a = Roamer::new("miyora.ember", "2026-11-01");
        let mut b = Roamer::new("miyora.ember", "2026-11-01");
        let n = ticks(ROAM_IDLE_BEFORE_WALK) * 3;
        assert_eq!(
            run(&mut a, n, (400.0, 200.0), PetState::Idle),
            run(&mut b, n, (400.0, 200.0), PetState::Idle)
        );
    }

    #[test]
    fn busy_states_and_freezes_stop_walking_and_reset_idle() {
        let mut roamer = Roamer::new("miyora.aoi", "2026-11-01");
        let walking = run(
            &mut roamer,
            ticks(ROAM_IDLE_BEFORE_WALK) + 5,
            (500.0, 300.0),
            PetState::Idle,
        );
        assert!(walking.last().expect("ticks").walking);
        for state in [
            PetState::Working,
            PetState::Thinking,
            PetState::Waiting,
            PetState::Paused,
            PetState::Sleeping,
            PetState::Error,
        ] {
            let output = roamer.step(RoamInput {
                state,
                frozen: false,
                position: (500.0, 300.0),
                allowed: allowed(),
            });
            assert!(!output.walking, "{state:?}");
        }
        let frozen = roamer.step(RoamInput {
            state: PetState::Idle,
            frozen: true,
            position: (500.0, 300.0),
            allowed: allowed(),
        });
        assert!(!frozen.walking);
        // The idle timer restarted, so the next idle tick does not walk.
        assert!(!run(&mut roamer, 1, (500.0, 300.0), PetState::Idle)[0].walking);
    }

    #[test]
    fn out_of_bounds_positions_clamp_and_tiny_work_areas_collapse() {
        let mut roamer = Roamer::new("miyora.aoi", "2026-11-01");
        let bounds = allowed();
        let output = run(&mut roamer, 1, (-5000.0, 9000.0), PetState::Idle);
        assert_eq!(output[0].position, (bounds.x, bounds.y + bounds.height));
        let tiny = LogicalRect::allowed_origins(
            LogicalRect {
                x: 10.0,
                y: 10.0,
                width: 100.0,
                height: 100.0,
            },
            270.0,
            340.0,
        );
        assert_eq!((tiny.width, tiny.height), (0.0, 0.0));
    }
}

use std::cmp::{max, min};

use datasize::DataSize;
use tracing::error;

use casper_types::{TimeDiff, Timestamp};

use crate::components::consensus::{
    highway_core::{
        finality_detector::{
            assigned_weight_and_latest_unit, find_max_quora, round_participation,
            RoundParticipation,
        },
        state, State,
    },
    traits::Context,
    utils::ValidatorIndex,
};

/// The number of most recent blocks for which we average the max quorum in which we participated.
const BLOCKS_TO_CONSIDER: usize = 5;
/// The average max quorum that triggers us to slow down: with this big or smaller average max
/// quorum per `BLOCKS_TO_CONSIDER`, we increase our round length.
const SLOW_DOWN_THRESHOLD: f64 = 0.8;
/// The average max quorum that triggers us to speed up: with this big or larger average max quorum
/// per `BLOCKS_TO_CONSIDER`, we decrease our round length.
const ACCELERATION_THRESHOLD: f64 = 0.9;
/// We will try to accelerate (decrease our round length) every `ACCELERATION_PARAMETER` rounds if
/// we have a big enough average max quorum.
const ACCELERATION_PARAMETER: u64 = 40;

#[derive(DataSize, Debug, Clone)]
pub(crate) struct PerformanceMeter {
    our_vid: ValidatorIndex,
    min_round_len: TimeDiff,
    max_round_len: TimeDiff,
    current_round_len: TimeDiff,
    last_switch_round_id: Timestamp,
}

impl PerformanceMeter {
    pub fn new(
        our_vid: ValidatorIndex,
        round_len: TimeDiff,
        min_round_len: TimeDiff,
        max_round_len: TimeDiff,
        timestamp: Timestamp,
    ) -> Self {
        let current_round_id = state::round_id(timestamp, round_len);
        Self {
            our_vid,
            min_round_len,
            max_round_len,
            current_round_len: round_len,
            last_switch_round_id: current_round_id,
        }
    }

    pub fn current_round_len(&self) -> TimeDiff {
        self.current_round_len
    }

    /// If the current timestamp indicates that the round has ended, checks the known proposals for
    /// a level-1 summit.
    /// If there is a summit, the round is considered successful. Otherwise, it is considered
    /// failed.
    /// Next, a number of last rounds are being checked for success and if not enough of them are
    /// successful, we return a higher round length for the future.
    /// If the length shouldn't grow, and the round ID is divisible by a certain number, a lower
    /// round length is returned.
    pub fn calculate_new_length<C: Context>(&mut self, state: &State<C>) -> TimeDiff {
        let panorama = state.panorama();
        let latest_block = match state.fork_choice(panorama) {
            Some(block) => block,
            None => {
                // we have no blocks to check - just return the current setting
                return self.current_round_len;
            }
        };

        let blocks_to_check = state.ancestor_hashes(latest_block);

        let max_quora: Vec<_> = blocks_to_check
            .take_while(|block| state.unit(block).round_id() >= self.last_switch_round_id)
            .filter_map(|block| {
                let round_id = state.unit(block).round_id();
                (!matches!(
                    round_participation(state, &panorama[self.our_vid], round_id),
                    RoundParticipation::Unassigned
                ))
                .then(|| {
                    let (assigned_weight, latest_units) =
                        assigned_weight_and_latest_unit(state, panorama, round_id);
                    let max_quorum = find_max_quora(state, block, &latest_units)
                        .get(self.our_vid)
                        .copied()
                        .unwrap_or(0u64.into());
                    max_quorum.0 as f64 / assigned_weight.0 as f64
                })
            })
            .take(BLOCKS_TO_CONSIDER)
            .collect();

        if max_quora.len() < BLOCKS_TO_CONSIDER {
            return self.current_round_len;
        }

        let avg_max_quorum = max_quora.iter().sum::<f64>() / max_quora.len() as f64;

        let current_round_id = state.unit(latest_block).round_id();
        let current_round_index = round_index(current_round_id, self.current_round_len);

        #[allow(clippy::integer_arithmetic)]
        if avg_max_quorum < SLOW_DOWN_THRESHOLD {
            self.current_round_len = min(self.current_round_len * 2, self.max_round_len);
            self.last_switch_round_id = current_round_id;
        } else if avg_max_quorum > ACCELERATION_THRESHOLD
            && current_round_index % ACCELERATION_PARAMETER == 0
        {
            self.current_round_len = max(self.current_round_len / 2, self.min_round_len);
            self.last_switch_round_id = current_round_id;
        }

        self.current_round_len
    }

    /// Returns an instance of `Self` for the new era: resetting the counters where appropriate.
    pub fn next_era(&self) -> Self {
        Self {
            our_vid: self.our_vid,
            min_round_len: self.min_round_len,
            max_round_len: self.max_round_len,
            current_round_len: self.current_round_len,
            last_switch_round_id: self.last_switch_round_id,
        }
    }
}

/// Returns the round index `i`, if `r_id` is the ID of the `i`-th round after the epoch.
#[allow(clippy::integer_arithmetic)] // Checking for division by 0.
fn round_index(r_id: Timestamp, round_len: TimeDiff) -> u64 {
    if round_len.millis() == 0 {
        error!("called round_index with round_len 0.");
        return r_id.millis();
    }
    r_id.millis() / round_len.millis()
}

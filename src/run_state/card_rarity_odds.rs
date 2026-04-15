use enum_map::{Enum, EnumMap};
use strum::{Display, EnumIter};

use crate::combat_state::cards::Rarity;

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Enum, EnumIter)]
enum Distribution {
    Regular,
    Elite,
    Boss,
    Shop,
    Uniform,
}

const BASE_ODDS: EnumMap<Distribution, EnumMap<Rarity, f32>> = EnumMap::from_array([
    //      Basic, Common, Uncommon, Rare, Special

    // Regular
    EnumMap::from_array([0.0, 0.6, 0.37, 0.03, 0.0]),
    // Elite
    EnumMap::from_array([0.0, 0.5, 0.4, 0.1, 0.0]),
    // Boss
    EnumMap::from_array([0.0, 0.0, 0.0, 1.0, 0.0]),
    // Shop
    EnumMap::from_array([0.0, 0.54, 0.37, 0.09, 0.0]),
    // Uniform
    EnumMap::from_array([0.0, 0.33, 0.33, 0.33, 0.0]),
]);

const BASE_ODDS_SCARCITY_ASCENSION: EnumMap<Distribution, EnumMap<Rarity, f32>> =
    EnumMap::from_array([
        //      Basic, Common, Uncommon, Rare, Special

        // Regular
        EnumMap::from_array([0.0, 0.615, 0.37, 0.015, 0.0]),
        // Elite
        EnumMap::from_array([0.0, 0.55, 0.4, 0.05, 0.0]),
        // Boss
        EnumMap::from_array([0.0, 0.0, 0.0, 1.0, 0.0]),
        // Shop
        EnumMap::from_array([0.0, 0.585, 0.37, 0.045, 0.0]),
        // Uniform
        EnumMap::from_array([0.0, 0.33, 0.33, 0.33, 0.0]),
    ]);

const BASE_PITY: f32 = -0.05;
const MAX_PITY: f32 = 0.4;
const PITY_RARITY_GROWTH: f32 = 0.01;
const PITY_RARITY_GROWTH_SCARCITY_ASCENSION: f32 = 0.005;

struct CardRarityOdds {
    current_pity_rare_offset: f32,
}

impl CardRarityOdds {
    pub fn roll_with_pity_and_advance_on_fail(
        &mut self,
        distribution: Distribution,
        scarcity_active: bool,
    ) -> Rarity {
        let result = self.roll_with_pity_without_changing_pity(distribution, scarcity_active);

        if result == Rarity::Rare {
            // Reset pity on rare
            self.current_pity_rare_offset = BASE_PITY;
        } else {
            // On Uncommon/Common increase pity

            let pity_growth = if scarcity_active {
                PITY_RARITY_GROWTH_SCARCITY_ASCENSION
            } else {
                PITY_RARITY_GROWTH
            };

            self.current_pity_rare_offset =
                if self.current_pity_rare_offset + pity_growth < MAX_PITY {
                    self.current_pity_rare_offset + pity_growth
                } else {
                    MAX_PITY
                }
        }

        result
    }

    pub fn roll_with_pity_without_changing_pity(
        &self,
        distribution: Distribution,
        scarcity_active: bool,
    ) -> Rarity {
        if distribution == Distribution::Boss {
            // Pity does not affect boss rewards
            Self::roll_with_rare_chance_offset(distribution, scarcity_active, 0.0)
        } else {
            // Use pity
            Self::roll_with_rare_chance_offset(
                distribution,
                scarcity_active,
                self.current_pity_rare_offset,
            )
        }
    }

    // Roll functions are only for reference
    fn roll_with_rare_chance_offset(
        distribution: Distribution,
        scarcity_active: bool,
        rare_chance_offset: f32,
    ) -> Rarity {
        let rolled: f32 = rand::random();

        let base_odds_rare =
            get_base_odds(distribution, Rarity::Rare, scarcity_active) + rare_chance_offset;

        if rolled < base_odds_rare {
            return Rarity::Rare;
        }

        if rolled < get_base_odds(distribution, Rarity::Uncommon, scarcity_active) + base_odds_rare
        {
            return Rarity::Uncommon;
        }

        Rarity::Common
    }
}

fn get_base_odds(distribution: Distribution, rarity: Rarity, scarcity_active: bool) -> f32 {
    if scarcity_active {
        BASE_ODDS_SCARCITY_ASCENSION[distribution][rarity]
    } else {
        BASE_ODDS[distribution][rarity]
    }
}

#[cfg(test)]
mod test {
    use std::io;

    use itertools::Itertools;
    use rayon::iter::{ParallelBridge, ParallelIterator};
    use strum::IntoEnumIterator;

    use super::*;

    #[test]
    fn experimentally_get_odds() {
        const NUM_SAMPLES: i32 = 100_000_000;

        if cfg!(debug_assertions) {
            eprintln!("SKIPPED CARD ODDS EXPERIMENT, run in release mode to get results");
            return;
        }

        let mut wtr = csv::Writer::from_writer(io::stdout());

        let max_num_pity: u16 = (((MAX_PITY - BASE_PITY) / PITY_RARITY_GROWTH) as u32 + 10)
            .try_into()
            .expect("Too much pity needed");
        let mut no_scarcity: Vec<_> = (0..max_num_pity)
            .map(|pity_count| {
                let uncapped = BASE_PITY + f32::from(pity_count) * PITY_RARITY_GROWTH;

                let capped = if uncapped > MAX_PITY {
                    MAX_PITY
                } else {
                    uncapped
                };

                (pity_count, capped)
            })
            .cartesian_product(Distribution::iter())
            .par_bridge()
            .map(|((pity_count, pity_value), distribution)| {
                let odds = CardRarityOdds {
                    current_pity_rare_offset: pity_value,
                };

                let mut res = EnumMap::from_fn(|_| 0);

                for _ in 0..NUM_SAMPLES {
                    let rarity = odds.roll_with_pity_without_changing_pity(distribution, false);

                    res[rarity] += 1;
                }

                (pity_count, res, distribution)
            })
            .collect();

        wtr.write_record([
            "scarcity_ascension_active",
            "reward_source",
            "pity_count",
            "rarity",
            "odds",
        ])
        .unwrap();

        no_scarcity.sort_by(|a, b| a.2.cmp(&b.2).then(a.0.cmp(&b.0)));

        for (pity_count, res, distribution) in no_scarcity {
            for (rarity, sample_count) in res {
                wtr.write_record([
                    format!("{}", false),
                    format!("{}", distribution),
                    format!("{}", pity_count),
                    format!("{}", rarity),
                    format!("{}", sample_count as f32 / NUM_SAMPLES as f32),
                ])
                .unwrap();
            }
        }

        let max_num_pity: u16 =
            (((MAX_PITY - BASE_PITY) / PITY_RARITY_GROWTH_SCARCITY_ASCENSION) as u32 + 10)
                .try_into()
                .expect("Too much pity needed");
        let mut with_scarcity: Vec<_> = (0..max_num_pity)
            .map(|pity_count| {
                let uncapped =
                    BASE_PITY + f32::from(pity_count) * PITY_RARITY_GROWTH_SCARCITY_ASCENSION;

                let capped = if uncapped > MAX_PITY {
                    MAX_PITY
                } else {
                    uncapped
                };

                (pity_count, capped)
            })
            .cartesian_product(Distribution::iter())
            .par_bridge()
            .map(|((pity_count, pity_value), distribution)| {
                let odds = CardRarityOdds {
                    current_pity_rare_offset: pity_value,
                };

                let mut res = EnumMap::from_fn(|_| 0);

                for _ in 0..NUM_SAMPLES {
                    let rarity = odds.roll_with_pity_without_changing_pity(distribution, true);

                    res[rarity] += 1;
                }

                (pity_count, res, distribution)
            })
            .collect();

        with_scarcity.sort_by(|a, b| a.2.cmp(&b.2).then(a.0.cmp(&b.0)));

        for (pity_count, res, distribution) in with_scarcity {
            for (rarity, sample_count) in res {
                wtr.write_record([
                    format!("{}", true),
                    format!("{}", distribution),
                    format!("{}", pity_count),
                    format!("{}", rarity),
                    format!("{}", sample_count as f32 / NUM_SAMPLES as f32),
                ])
                .unwrap();
            }
        }

        wtr.flush().unwrap();
    }
}

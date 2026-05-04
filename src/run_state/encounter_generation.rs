use strum::IntoEnumIterator;

use crate::distribution::{Distribution, DistributionFamily};
use crate::{combat_state::encounter::EncounterPrototype, run_state::ActPrototype};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct EncounterGenerationState {
    past_encounters: Vec<EncounterPrototype>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EncounterKind {
    Boss,
    Elite,
    Normal,
    Weak,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EncounterGenerationKind {
    Boss,
    Elite,
    Hallway,
}

pub struct EncounterGenerationOptions {
    pub kind: EncounterGenerationKind,

    pub act: ActPrototype,
}

impl ActPrototype {
    fn num_weaks(&self) -> usize {
        match self {
            ActPrototype::Overgrowth => 3,
            ActPrototype::Underdocks => 3,
            ActPrototype::Hive => 2,
            ActPrototype::Glory => 2,
        }
    }
}

impl EncounterGenerationState {
    pub fn next_encounter<Family: DistributionFamily>(
        self,
        options: EncounterGenerationOptions,
    ) -> Family::Distribution<(Self, EncounterPrototype)> {
        let kind = match options.kind {
            EncounterGenerationKind::Boss => EncounterKind::Boss,
            EncounterGenerationKind::Elite => EncounterKind::Elite,
            EncounterGenerationKind::Hallway => {
                if self
                    .past_encounters
                    .iter()
                    .filter(|encounter| {
                        encounter.get_act() == options.act
                            && encounter.get_kind() == EncounterKind::Weak
                    })
                    .count()
                    >= options.act.num_weaks()
                {
                    EncounterKind::Normal
                } else {
                    EncounterKind::Weak
                }
            }
        };

        let mut options = EncounterPrototype::iter()
            .filter(|encounter| encounter.get_act() == options.act && encounter.get_kind() == kind)
            .filter(|encounter| encounter.is_finished_implementing())
            .filter(|encounter| !self.past_encounters.contains(encounter))
            .peekable();

        if options.peek().is_none() {
            return Family::Distribution::<(_, _)>::single_value((
                self,
                EncounterPrototype::Byrdonis,
            ));
            todo!("All encounters used already: {:?}", &self.past_encounters)
        }

        // TODO: Are the encounters equally likely?
        Family::Distribution::<(_, _)>::equal_chance(options.map(|encounter| {
            // FIXME: This clone is bad, since it will clone for all valid options even when only one is needed for Distribution::single
            let mut state = self.clone();
            state.past_encounters.push(encounter);

            (state, encounter)
        }))
    }
}

use strum::IntoEnumIterator;

use crate::{
    distribution::{Distribution, DistributionFamily},
    run_state::{
        Act,
        events::{EventLocationInfo, EventPrototype},
    },
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct EventGenerationState {
    past_events: Vec<EventPrototype>,
}

impl EventGenerationState {
    pub fn next_event<Family: DistributionFamily>(
        self,
        current_act: &Act,
        event_location: EventLocationInfo,
    ) -> Family::Distribution<(Self, EventPrototype)> {
        let mut options = EventPrototype::iter()
            .filter(|event| event.can_spawn(current_act, event_location))
            .filter(|event| !self.past_events.contains(event))
            .peekable();

        if options.peek().is_none() {
            todo!("All events used already: {:?}", &self.past_events)
        }

        // TODO: Are the events equally likely?
        Family::Distribution::<(_, _)>::equal_chance(options.map(|event| {
            // FIXME: This clone is bad, since it will clone for all valid options even when only one is needed for Distribution::single
            let mut state = self.clone();
            state.past_events.push(event);

            (state, event)
        }))
    }
}

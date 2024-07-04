#[derive(Debug)]
pub struct Trigger {
    /// The indices of the first and last value of an event that are not zero.
    /// TODO: For some things like RF we might want to include the zero, for
    /// others like ADC we do not as those are not samples - think about timing!
    events: Vec<(usize, usize)>,
}

impl Trigger {
    pub fn new(samples: &[f64]) -> Self {
        let mut starts = Vec::new();
        let mut ends = Vec::new();

        // Trigger window size - amount of zeros that separate pulses
        const WND: usize = 10;
        let n_samples = samples.len();
        // TODO: return error or empty Trigger?
        assert!(n_samples > WND);

        // There might be less zeros before the first start
        if let Some(i) = samples.iter().take(WND - 1).position(|&x| x != 0.0) {
            starts.push(i);
        }

        // 8 consecutive 0s count as an start / end
        for (i, w) in samples.windows(WND).enumerate() {
            if w[0..WND - 1].iter().all(|&x| x == 0.0) && w[WND - 1] != 0.0 {
                starts.push(i + WND - 1);
            }
            if w[0] != 0.0 && w[1..WND].iter().all(|&x| x == 0.0) {
                ends.push(i);
            }
        }

        // There might be less zeros after the last end
        if let Some(i) = samples.iter().rev().take(WND - 1).position(|&x| x != 0.0) {
            ends.push((n_samples - i).min(n_samples - 1));
        }

        // Logic bug in this code if any of this triggers
        assert_eq!(starts.len(), ends.len());

        let events: Vec<(usize, usize)> = starts
            .into_iter()
            .zip(ends)
            .map(|(start, end)| {
                assert!(start < end);
                (start, end)
            })
            .collect();

        // Check if sorted and no overlap - w[x].0 < w[x].1 is guaranteed above
        assert!(events.windows(2).all(|w| w[0].1 < w[1].0));

        Self { events }
    }

    pub fn search(&self, i_start: usize) -> Option<(usize, usize)> {
        match self
            .events
            .binary_search_by_key(&i_start, |&(start, _)| start)
        {
            // we are exactly on the starting point of the event
            Ok(idx) => Some(self.events[idx]),
            // we must check if we are before the event
            Err(idx) => {
                if idx < self.events.len() && i_start < self.events[idx].0 {
                    Some(self.events[idx])
                } else {
                    None
                }
            }
        }
    }

    pub fn events(&self, i_start: usize, i_end: usize) -> impl Iterator<Item = (usize, usize)> + '_ {
        // Index of the first event overlapping with the time range
        let idx = match self
            .events
            .binary_search_by_key(&i_start, |&(start, _)| start)
        {
            Ok(idx) => idx,
            Err(idx) => {
                if idx > 0 && self.events[idx - 1].1 > i_start {
                    idx - 1
                } else {
                    idx
                }
            }
        };

        // TODO: is this correct? evt_start and evt_end values are inclusive currently!

        self.events[idx..]
            .iter()
            .take_while(move |&&(evt_start, evt_end)| i_start < evt_end && evt_start < i_end)
            .copied()
    }
}

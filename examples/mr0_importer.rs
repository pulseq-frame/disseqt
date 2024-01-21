// This is only a mock importer to test performance, we don't have a mr0-rs (yet?)
// It mimicks the python example importer

use disseqt::EventType;

fn import_pulseq(path: &str) -> mr0::Sequence {
    let parser = disseqt::load_pulseq(path).unwrap();
    let mut seq = mr0::Sequence::default();
    let mut t = 0.0;

    fn pulse_usage(angle: f32) -> mr0::PulseUsage {
        if angle.abs() < 100.0 * std::f32::consts::PI / 180.0 {
            mr0::PulseUsage::Excit
        } else {
            mr0::PulseUsage::Refoc
        }
    }

    let fov = parser.fov().unwrap_or((1.0, 1.0, 1.0));

    while let Some((pulse_start, pulse_end)) = parser.encounter(t, EventType::RfPulse) {
        let rep_start = (pulse_start + pulse_end) / 2.0;

        // Calculate end of repetition
        let rep_end = match parser.encounter(pulse_end, EventType::RfPulse) {
            Some((start, end)) => (start + end) / 2.0,
            None => parser.duration(),
        };

        // Get all ADC sample times
        let adc_times = parser.events(EventType::Adc, rep_start, rep_end, usize::MAX);
        if let Some(last_sample) = adc_times.last() {
            t = *last_sample;
        }

        // Now build the mr0 repetition

        let rep = seq.new_rep(adc_times.len() + 1);
        let moment = parser.integrate_one(pulse_start, pulse_end);
        rep.pulse.angle = moment.pulse.angle;
        rep.pulse.phase = moment.pulse.phase;
        rep.pulse.usage = pulse_usage(moment.pulse.angle);

        let abs_times: Vec<f32> = std::iter::once(&rep_start)
            .chain(adc_times.iter())
            .chain(std::iter::once(&rep_end))
            .cloned()
            .collect();

        // NEW: using the new API, we don't need to call these functions for every
        // single time step, but only once. They also could be optimized internally
        // to avoid recalculating everything for every single sample.
        let moments = parser.integrate(&abs_times);
        let samples = parser.sample(&adc_times);

        // With the new API, these loops could probably be simplified
        for i in 0..abs_times.len() - 1 {
            rep.events[i].dur = abs_times[i + 1] - abs_times[i];

            let gradm = moments[i].gradient;
            rep.events[i].gradm = [gradm.gx * fov.0, gradm.gy * fov.1, gradm.gz * fov.2];

            // There is no ADC at the end of the last sample
            if i < adc_times.len() {
                rep.events[i].adc_usage = 1;
                // Last event goes to start of next rep, doesn't have an ADC
                let adc = samples[i].adc;
                rep.events[i].adc_phase = std::f32::consts::FRAC_PI_2 - adc.phase;
            }
        }
    }

    seq
}

fn main() {
    let start = std::time::Instant::now();
    std::hint::black_box(import_pulseq("examples/gre.seq"));
    let end = std::time::Instant::now();
    println!("Importing took {} seconds", (end - start).as_secs_f32());
}

mod mr0 {
    #[derive(Default, Clone, Copy)]
    pub enum PulseUsage {
        Excit,
        Refoc,
        #[default]
        Undefined,
    }

    #[derive(Default, Clone)]
    pub struct Sequence {
        pub reps: Vec<Repetition>,
    }

    #[derive(Default, Clone)]
    pub struct Repetition {
        pub pulse: Pulse,
        pub events: Vec<Event>,
    }

    #[derive(Default, Clone, Copy)]
    pub struct Pulse {
        pub angle: f32,
        pub phase: f32,
        pub usage: PulseUsage,
    }

    #[derive(Default, Clone, Copy)]
    pub struct Event {
        pub dur: f32,
        pub gradm: [f32; 3],
        pub adc_phase: f32,
        pub adc_usage: u32,
    }

    impl Sequence {
        pub fn new_rep(&mut self, len: usize) -> &mut Repetition {
            let rep = Repetition {
                pulse: Pulse::default(),
                events: vec![Event::default(); len],
            };
            self.reps.push(rep);
            self.reps.last_mut().unwrap()
        }
    }
}

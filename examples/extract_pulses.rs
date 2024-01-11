use disseqt::{Poi, Sequence};

fn main() {
    let source = std::fs::read_to_string("examples/gre.seq").unwrap();
    let seq = Sequence::from_pulseq_file(&source).unwrap();

    let mut t = 0.0;
    loop {
        if let Some(pulse_start) = seq.next(t, Poi::PulseStart) {
            let pulse_end = seq.next(pulse_start, Poi::PulseEnd).unwrap();
            let (pulse, _) = seq.integrate(pulse_start, pulse_end);

            println!("[{pulse_start}: {}ms]: {pulse:?}", (pulse_end - pulse_start) * 1e3);
            t = pulse_end;
        } else {
            // No more pulses in the sequence
            break;
        }
    }
}

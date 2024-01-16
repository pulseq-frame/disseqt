use disseqt::{EventType, Sequence};

fn main() {
    let source = std::fs::read_to_string("examples/gre.seq").unwrap();
    let seq = Sequence::from_pulseq_file(&source).unwrap();

    let mut t = 0.0;
    while let Some((pulse_start, pulse_end)) = seq.next_block(t, EventType::RfPulse) {
        let (pulse, _) = seq.integrate(pulse_start, pulse_end);

        println!(
            "[{pulse_start}: {}ms]: {pulse:?}",
            (pulse_end - pulse_start) * 1e3
        );
        t = pulse_end;
    }
}

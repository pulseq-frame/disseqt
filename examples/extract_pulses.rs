use disseqt::EventType;

fn main() {
    let seq = disseqt::load_pulseq("examples/gre.seq").unwrap();

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

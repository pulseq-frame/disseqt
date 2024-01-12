use disseqt::{Poi, Sequence};

fn main() {
    let source = std::fs::read_to_string("examples/gre.seq").unwrap();
    let seq = Sequence::from_pulseq_file(&source).unwrap();

    let t_start = seq.next(0.0, Poi::PulseStart).unwrap();
    let t_end = seq.next(0.0, Poi::PulseEnd).unwrap(); // / 10.0;

    let mut t = t_start;
    let mut sample_count = 0;
    while let Some(t_sample) = seq.next(t, Poi::PulseSample) {
        if t_sample > t_end {
            break;
        } else {
            t = t_sample + 1e-7; // Advance a bit past the sample
            sample_count += 1;
        }
    }
    println!("First pulse: [{t_start}..{t_end}] s, {sample_count} samples");
}

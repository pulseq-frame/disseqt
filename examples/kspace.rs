use disseqt::{Poi, Sequence};

fn main() {
    let source = std::fs::read_to_string("examples/gre.seq").unwrap();
    let seq = Sequence::from_pulseq_file(&source).unwrap();

    let mut kspace: Vec<Vec<(f32, f32)>> = Vec::new();
    let mut t = 0.0;

    while let Some(pulse_start) = seq.next(t, Poi::PulseStart) {
        // Start integrating at the center of the pulse
        let pulse_end = seq.next(pulse_start, Poi::PulseEnd).unwrap();
        t = (pulse_start + pulse_end) / 2.0;

        let mut kx = 0.0;
        let mut ky = 0.0;
        kspace.push(Vec::new());
        let line = kspace.last_mut().unwrap();

        let rep_end = seq.next(pulse_end + 1e-7, Poi::PulseStart).unwrap_or(seq.duration());
        while let Some(next_adc) = seq.next(t, Poi::AdcSample) {
            if next_adc > rep_end {
                break;
            }

            let (_, grad) = seq.integrate(t, next_adc);
            t = next_adc + 1e-6;

            // TODO: allow to extract FOV from seq if available
            kx += grad.gx * 0.256;
            ky += grad.gy * 0.256;
            line.push((kx, ky));
        }
    }

    // TODO: k-coordinates don't seem to be correct -> investigate!
    let kx: Vec<f32> = kspace[0].iter().map(|(x, _)| *x).collect();
    let ky: Vec<f32> = kspace.iter().map(|line| line[0].1).collect();
    println!("{kx:?}");
    println!("{ky:?}");
}

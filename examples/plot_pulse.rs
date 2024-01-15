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

    // Sample the pulse
    let plot_width = 50;
    let plot_height = 30;
    let mut samples = Vec::new();

    for t in 0..plot_width {
        let t = (t as f32 + 0.5) / plot_width as f32;
        let t = t_start + (t_end - t_start) * t;

        let (pulse, _, _) = seq.sample(t);
        samples.push(pulse.amplitude * pulse.phase.cos());
    }

    // Plotting code
    let min = samples
        .iter()
        .cloned()
        .min_by(|a, b| a.total_cmp(b))
        .unwrap();
    let max = samples
        .iter()
        .cloned()
        .max_by(|a, b| a.total_cmp(b))
        .unwrap();

    for i in 0..=plot_height {
        let y = max - (max - min) * (i as f32 / plot_height as f32);
        print!("{y:-8.2} | ");

        for &sample in &samples {
            if (y > 0.0) != (y >= sample) {
                print!("█");
            } else {
                print!(" ");
            }
        }
        println!()
    }
}

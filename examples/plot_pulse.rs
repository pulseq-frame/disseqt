use disseqt::EventType;

fn main() {
    // let seq = disseqt::load_pulseq("examples/gre.seq").unwrap();
    let seq = disseqt::load_dsv("examples/3DSnapshotGRE_Comparision_E_0_64_64_8_alternating_fully_sampled/SimulationProtocol").unwrap();

    let (t_start, t_end) = seq.encounter(0.0, EventType::RfPulse).unwrap();

    let mut t = t_start;
    let mut sample_count = 0;
    while let Some(t_sample) = seq.next_event(t, EventType::RfPulse) {
        if t_sample > t_end {
            break;
        } else {
            t = t_sample + 1e-7; // Advance a bit past the sample
            sample_count += 1;
        }
    }
    println!("First pulse: [{t_start}..{t_end}] s, {sample_count} Events");

    // Sample the pulse
    let plot_width = 50;
    let plot_height = 30;
    let mut samples = Vec::new();

    for t in 0..plot_width {
        let t = (t as f64 + 0.5) / plot_width as f64;
        let t = t_start + (t_end - t_start) * t;

        let sample = seq.sample_one(t);
        samples.push(sample.pulse.amplitude * sample.pulse.phase.cos());
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
        let y = max - (max - min) * (i as f64 / plot_height as f64);
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

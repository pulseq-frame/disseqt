use disseqt::EventType;

fn main() {
    let seq = disseqt::load_pulseq("examples/gre.seq").unwrap();
    let fov = seq.fov().unwrap_or((1.0, 1.0, 1.0));

    let mut kspace: Vec<Vec<(f64, f64, f64)>> = Vec::new();
    let mut t = 0.0;

    while let Some((pulse_start, pulse_end)) = seq.encounter(t, EventType::RfPulse) {
        // Start integrating at the center of the pulse
        t = (pulse_start + pulse_end) / 2.0;

        let mut kx = 0.0;
        let mut ky = 0.0;
        let mut kz = 0.0;
        kspace.push(Vec::new());
        let line = kspace.last_mut().unwrap();

        let (_, adc_end) = seq.encounter(t, EventType::Adc).unwrap();
        while let Some(next_adc) = seq.next_event(t + 1e-6, EventType::Adc) {
            if next_adc > adc_end {
                break;
            }

            let moment = seq.integrate_one(t, next_adc);
            t = next_adc;

            kx += moment.gradient.x * fov.0;
            ky += moment.gradient.y * fov.1;
            kz += moment.gradient.z * fov.2;
            line.push((kx, ky, kz));
        }
    }

    let kx: Vec<f64> = kspace[0].iter().map(|(x, _, _)| *x).collect();
    let ky: Vec<f64> = kspace.iter().map(|line| line[0].1).collect();
    let kz: Vec<f64> = kspace.iter().map(|line| line[0].2).collect();
    println!("{kx:?}");
    println!("{ky:?}");
    println!("{kz:?}");
}

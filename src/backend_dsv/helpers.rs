use std::{collections::HashMap, path::Path};

use super::Error;

pub struct DsvFile {
    pub definitions: HashMap<String, String>,
    pub values: Vec<i64>,
}

impl DsvFile {
    pub fn load<P: AsRef<Path>>(path: P, which_dsv: &str) -> Result<Self, Error> {
        let file_name = path.as_ref().file_stem().unwrap().to_str().unwrap();
        let file_path = path
            .as_ref()
            .with_file_name(format!("{file_name}_{which_dsv}.dsv"));
        let file_buf =
            std::fs::read(file_path.clone()).map_err(|_| Error::FileNotFound(file_path))?;
        let file_str = String::from_utf8_lossy(&file_buf);

        let definitions_raw = file_str
            .split('[')
            .find(|s| s.starts_with("DEFINITIONS"))
            .unwrap();

        let definitions: HashMap<_, _> = definitions_raw
            .lines()
            .skip(1)
            .filter(|l| !l.is_empty())
            .map_while(|def| def.split_once('=')) // stop at first non-def
            .map(|(key, val)| (key.trim().to_owned(), val.trim().to_owned()))
            .collect();

        let values_raw = file_str
            .split('[')
            .find(|s| s.starts_with("VALUES"))
            .unwrap();

        let values: Vec<i64> = values_raw
            .lines()
            .skip(1)
            .map_while(|s| s.parse().ok())
            .collect();

        Ok(Self {
            definitions,
            values,
        })
    }

    // TODO: don't unwrap but return the parse errors
    // TODO: do the same with key errors (currently panics)

    pub fn time_step(&self) -> f64 {
        let time_unit = hori_unit_si_factor(&self.definitions["HORIUNITNAME"]);
        let time_step = self.definitions["HORIDELTA"].parse::<f64>().unwrap();
        time_step * time_unit
    }

    pub fn amp_step(&self, ref_voltage: Option<f64>) -> f64 {
        let amp_unit = vert_unit_si_factor(&self.definitions["VERTUNITNAME"], ref_voltage);
        let amp_step = 1.0 / self.definitions["VERTFACTOR"].parse::<f64>().unwrap();
        amp_step * amp_unit
    }
}

fn vert_unit_si_factor(unit: &str, ref_voltage: Option<f64>) -> f64 {
    const GAMMA: f64 = 42_576_385.43;
    const PI: f64 = std::f64::consts::PI;
    // 1ms Block pulse at ref_voltage = 180° -> ref_voltage = 500 Hz rotation

    match unit {
        // SI: [Hz/m]
        "T/m" => GAMMA,
        "mT/m" => 1e-3 * GAMMA,
        // SI: [rad]
        "Degree" => PI / 180.0,
        // SI: [Hz]
        "Volt" => 500.0 / ref_voltage.unwrap(), // optional if unit is not Volts
        // No unit (ADC)
        "-" => 1.0,
        _ => panic!("Unknown amplitude unit {unit:?}"),
    }
}

fn hori_unit_si_factor(unit: &str) -> f64 {
    match unit {
        "s" => 1e0,
        "ms" => 1e-3,
        "µs" | "μs" | "�s" | "us" => 1e-6,
        "ns" => 1e-9,
        _ => panic!("Unknown time unit {unit:?}"),
    }
}

pub fn decompress_shape(samples: Vec<i64>, num_samples: usize) -> Vec<i64> {
    // First, decompress into the deriviate of the shape
    let mut deriv = Vec::with_capacity(num_samples);

    // The two samples before the current one, to detect RLE
    let mut a = i64::MIN;
    let mut b = i64::MAX;
    // After a detected RLE, skip the RLE check for two samples
    let mut skip = 0;

    for sample in samples.into_iter() {
        if a == b && skip == 0 {
            skip = 2;
            for _ in 0..sample as usize {
                deriv.push(b);
            }
        } else {
            if skip > 0 {
                skip -= 1;
            }
            deriv.push(sample);
        }

        a = b;
        b = sample;
    }

    if deriv.len() != num_samples {
        panic!(
            "Wrong decompressed length: got {}, expected {}",
            deriv.len(),
            num_samples
        );
    }

    // Then, do a cumultative sum to get the shape
    deriv
        .into_iter()
        .scan(0, |acc, x| {
            *acc += x;
            Some(*acc)
        })
        .collect()
}

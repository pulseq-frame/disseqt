use crate::Backend;
use std::{collections::HashMap, path::Path};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {}

pub struct DsvSequence {
    rf_amplitude: [Vec<f64>; 2],
    // rf_phase: [Vec<f64>; 2],
}

impl DsvSequence {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        Ok(Self {
            rf_amplitude: load_rfds(path)?,
        })
    }
}

impl Backend for DsvSequence {
    fn fov(&self) -> Option<(f64, f64, f64)> {
        todo!()
    }

    fn duration(&self) -> f64 {
        todo!()
    }

    fn events(&self, ty: crate::EventType, t_start: f64, t_end: f64, max_count: usize) -> Vec<f64> {
        todo!()
    }

    fn encounter(&self, t_start: f64, ty: crate::EventType) -> Option<(f64, f64)> {
        todo!()
    }

    fn sample(&self, time: &[f64]) -> Vec<crate::Sample> {
        todo!()
    }

    fn integrate(&self, time: &[f64]) -> Vec<crate::Moment> {
        todo!()
    }
}

// TODO: replace all the unwraps with errors

fn load_rfds<P: AsRef<Path>>(path: P) -> Result<[Vec<f64>; 2], Error> {
    // TODO: We only load RDF 1 here, put this in separate function and load both channels
    let file_name = path.as_ref().file_stem().unwrap().to_str().unwrap();
    let file_path = path.as_ref().with_file_name(format!("{file_name}_RFD.dsv"));
    let file_buf = std::fs::read(file_path).unwrap();
    let file_str = String::from_utf8_lossy(&file_buf);

    let definitions = file_str
        .split('[')
        .find(|s| s.starts_with("DEFINITIONS"))
        .unwrap();

    let definitions: HashMap<_, _> = definitions
        .lines()
        .skip(1)
        .filter(|l| !l.is_empty())
        .map(|def| def.split_once('=').unwrap())
        .map(|(key, val)| (key.trim(), val.trim()))
        .collect();

    let values = file_str
        .split('[')
        .find(|s| s.starts_with("VALUES"))
        .unwrap();

    let values: Vec<i64> = values
        .lines()
        .skip(1)
        .map_while(|s| s.parse().ok())
        .collect();

    let num_samples: usize = definitions["SAMPLES"].parse().unwrap();

    let rf = decompress_shape(values, num_samples);

    println!("{definitions:#?}");
    println!("{:?}", &rf[1000250..1000400]);

    todo!()
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

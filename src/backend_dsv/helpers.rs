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
        let file_buf = std::fs::read(file_path).unwrap();
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

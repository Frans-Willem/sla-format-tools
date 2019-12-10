extern crate image;
extern crate nom;

mod formats;

use crate::formats::photons;
use image::{ImageBuffer, Pixel, Rgb, RgbImage};
use nom::{number::complete::*, sequence::tuple, IResult};
use std::fmt::Display;
use std::fs::File;
use std::io::{Read, Write};

/*
impl RLEBits {
    fn iter(&self) -> impl Iterator<Item = bool> + '_ {
        self.data
            .iter()
            .scan(self.num_ones, |ones_remaining, b| {
                let b = b.reverse_bits();
                let value = (b & 0x80) != 0;
                let mut repeat = ((b & 0x7F) as usize) + 1;
                if value {
                    if repeat > *ones_remaining {
                        repeat = *ones_remaining;
                    }
                    *ones_remaining = *ones_remaining - repeat;
                }
                Some(std::iter::repeat(value).take(repeat))
            })
            .flatten()
            .chain(std::iter::repeat(false))
    }

    fn to_vec(&self, len: usize) -> Vec<bool> {
    }
}
*/

fn main() -> std::io::Result<()> {
    let mut file = File::open("example_files/ArnoldOrczenegger.photons")?;
    let mut contents = Vec::new();
    let len = file.read_to_end(&mut contents)?;
    println!("Length: {}", len);
    let result = photons::parse::parse_photons_file(&contents);
    if let Ok((remaining, result)) = result {
        println!("Bytes remaining: {}", remaining.len());
        result.thumbnail.save("thumbnail.png").unwrap();
        println!("Num layers: {:?}", result.layers.len());
        for (index, layer) in result.layers.iter().enumerate() {
            println!("Layer {}...", index);
            let len = (layer.width * layer.height) as usize;
            let decompressed = layer.data.decompress(len);
            let decompressed_u8: Vec<u8> = decompressed
                .iter()
                .map(|v| if *v { 255 } else { 0 })
                .collect();
            let recompressed = photons::data::CompressedBitstream::compress(&decompressed);
            if layer.data != recompressed {
                println!("Recompression failure at layer {}", index);
                layer
                    .data
                    .debug_out(&mut File::create(format!("layer{}.pre.txt", index)).unwrap())
                    .unwrap();
                recompressed
                    .debug_out(&mut File::create(format!("layer{}.post.txt", index)).unwrap())
                    .unwrap();
                File::create(format!("layer{}.pre.bin", index))
                    .unwrap()
                    .write_all(&layer.data.data)
                    .unwrap();
                File::create(format!("layer{}.post.bin", index))
                    .unwrap()
                    .write_all(&recompressed.data)
                    .unwrap();
                File::create(format!("layer{}.data.bin", index))
                    .unwrap()
                    .write_all(&decompressed_u8)
                    .unwrap();
            }
        }
    } else {
        println!("Some kind of error parsing :(");
    }

    Ok(())
}

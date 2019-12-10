extern crate image;
extern crate nom;

mod formats;
mod parse_rgb565;

use crate::formats::photons;
use crate::formats::pws;
use image::{GrayImage, ImageBuffer, Pixel, Rgb, RgbImage};
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
    let mut file = File::open("example_files/ArnoldOrczenegger_v2.pws")?;
    let mut contents = Vec::new();
    let len = file.read_to_end(&mut contents)?;
    println!("Length: {}", len);
    let result = pws::parse::parse_pws_file(&contents);
    if let Ok((remaining, result)) = result {
        println!("Bytes remaining: {}", remaining.len());
        println!("Header: {:?}", result.header);
        let image_size: usize = result.header.width as usize * result.header.height as usize;
        for (index, layer) in result.layers.iter().enumerate() {
            println!("Layer {}...", index);
            let decompressed = layer.data.decompress();
            let recompressed = pws::data::CompressedBitstream::compress(&decompressed, image_size);
            if layer.data != recompressed {
                println!("Mismatch! {} {}", layer.data.0.len(), recompressed.0.len());
                File::create(format!("layer{}.pre.bin", index))
                    .unwrap()
                    .write_all(&layer.data.0)
                    .unwrap();
                File::create(format!("layer{}.post.bin", index))
                    .unwrap()
                    .write_all(&recompressed.0)
                    .unwrap();
                break;
            }
        }
    /*
    for index in 359..360 {
        let layer = &result.layers[index];
        let data = layer.data.decompress();
        let mut pixels = vec![0;image_size];
        for pixel_index in 1..image_size {
            let mut count : usize = 0;
            for bit in 0..result.header.bits_per_pixel as usize {
                if data[pixel_index + (bit * image_size)] {
                    count += 1;
                }
            }
            count *= 255;
            count /= result.header.bits_per_pixel as usize;
            pixels[pixel_index] = (count & 0xFF) as u8;
        }
        let img = GrayImage::from_vec(result.header.width, result.header.height, pixels).unwrap();
        img.save(format!("layer{}.png", index)).unwrap();
    }
    */
    } else {
        println!("Some kind of error parsing :(");
    }

    Ok(())
}

extern crate cookie_factory;
extern crate image;
extern crate nom;
//extern crate zip;
//

pub mod formats;
pub mod parse_rgb565;

/*
use crate::formats::photons;
use crate::formats::pws;
use image::{GrayImage, ImageBuffer, Pixel, Rgb, RgbImage};
use nom::{number::complete::*, sequence::tuple, IResult};
use std::fmt::Display;
use std::fs::File;
use std::io::{Read, Write};

fn main() -> std::io::Result<()> {
    let mut file = File::open("example_files/ArnoldOrczenegger_v2.pws")?;
    let mut contents = Vec::new();
    let len = file.read_to_end(&mut contents)?;
    println!("Length: {}", len);
    let result = pws::parse::parse_pws_file(&contents);
    if let Ok((remaining, result)) = result {
        println!("Bytes remaining: {}", remaining.len());
        println!("Header: {:?}", result.header);
        let mut f = File::create("dump.pws").unwrap();
        let (f, written) = cookie_factory::gen(pws::gen::gen_pws_file(&result), f).unwrap();
        println!("Written: {}", written);
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
*/

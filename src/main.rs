extern crate image;
extern crate nom;

use image::{ImageBuffer, Pixel, Rgb, RgbImage};
use nom::{number::complete::*, sequence::tuple, IResult};
use std::fmt::Display;
use std::fs::File;
use std::io::{Read, Write};

struct PhotonsFile {
    pixelsize: f64, // in mm
    layerheight: f64,
    exposure_time: f64,        // in sec
    off_time: f64,             // in sec
    bottom_exposure_time: f64, // in sec
    num_bottom_layers: u32,
    lift_distance: f64, // in mm
    lift_speed: f64,    // in mm/sec
    retract_speed: f64, // in mm/sec
    total_volume: f64,
    thumbnail: RgbImage,
    layers: Vec<PhotonsLayer>,
}

pub struct RLEBits {
    num_ones: usize,
    data: Vec<u8>,
}

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
        let mut ret = vec![false; len];
        let mut ones_remaining = self.num_ones;
        let mut index: usize = 0;
        for b in self.data.iter() {
            let b = b.reverse_bits();
            let value = (b & 0x80) != 0;
            let mut repeat: usize = ((b & 0x7F) as usize) + 1;
            if value {
                if repeat > ones_remaining {
                    repeat = ones_remaining;
                }
                ones_remaining = ones_remaining - repeat;
            }
            while repeat > 0 && index < ret.len() {
                ret[index] = value;
                repeat -= 1;
                index += 1;
            }
        }
        ret
    }
}

struct PhotonsLayer {
    width: u32,
    height: u32,
    data: RLEBits,
}

fn upscale_5bit_to_8bit(input: u8) -> u8 {
    (input << 3) | (input >> 2)
}

fn upscale_6bit_to_8bit(input: u8) -> u8 {
    (input << 2) | (input >> 4)
}

fn parse_photons_thumbnail_pixel(input: &[u8]) -> IResult<&[u8], Rgb<u8>> {
    let (input, data) = le_u16(input)?;
    Ok((
        input,
        Rgb([
            upscale_5bit_to_8bit((data & 0x1F) as u8),
            upscale_6bit_to_8bit(((data >> 5) & 0x3F) as u8),
            upscale_5bit_to_8bit(((data >> 11) & 0x1F) as u8),
        ]),
    ))
}

fn parse_photons_thumbnail(input: &[u8]) -> IResult<&[u8], RgbImage> {
    let (input, (width, unk1, height, unk2)) = tuple((be_u32, be_u32, be_u32, be_u32))(input)?;
    if unk1 != 42 || unk2 != 10 {
        println!("Unknowns in thumbnail unexpected: {} {}", unk1, unk2);
        return Err(nom::Err::Failure((input, nom::error::ErrorKind::Verify)));
    }
    let (input, pixels) =
        nom::multi::count(parse_photons_thumbnail_pixel, (width * height) as usize)(input)?;
    let pixels: Vec<u8> = pixels.iter().flat_map(|p| p.0.iter()).cloned().collect();
    Ok((input, ImageBuffer::from_vec(width, height, pixels).unwrap()))
}

fn parse_photons_layer(input: &[u8]) -> IResult<&[u8], PhotonsLayer> {
    let (input, (num_white, unknown1, width, height, total_size, width_revbits, height_revbits)) =
        tuple((be_u32, be_u64, be_u32, be_u32, be_u32, le_u16, le_u16))(input)?;
    if unknown1 != 0 {
        println!("Unknowns in layer unexpected: {:X}", unknown1);
        return Err(nom::Err::Failure((input, nom::error::ErrorKind::Verify)));
    }
    if width_revbits.reverse_bits() as u32 != width || height_revbits.reverse_bits() as u32 != height {
        println!("Reverse-bit fields invalid: {:X} != {:X} or {:X} != {:X}", width, width_revbits.reverse_bits(), height, height_revbits.reverse_bits());
        return Err(nom::Err::Failure((input, nom::error::ErrorKind::Verify)));
    }
    let (input, data) = nom::bytes::complete::take((total_size - 32) / 8)(input)?;
    let data = RLEBits {
        num_ones: num_white as usize,
        data: data.to_vec(),
    };
    Ok((
        input,
        PhotonsLayer {
            width,
            height,
            data,
        },
    ))
}

fn parse_photons_file(input: &[u8]) -> IResult<&[u8], PhotonsFile> {
    let (input, version) = be_u32(input)?;
    if version != 2 {
        println!("Unexpected version: {}", version);
        return Err(nom::Err::Failure((input, nom::error::ErrorKind::Verify)));
    }
    let (input, unk1) = be_u16(input)?;
    if unk1 != 0x31 {
        println!("Unexpected unknowns: 0x{:X}", unk1);
        return Err(nom::Err::Failure((input, nom::error::ErrorKind::Verify)));
    }
    let (
        input,
        (
            pixelsize,
            layerheight,
            exposure_time,
            off_time,
            bottom_exposure_time,
            num_bottom_layers,
            lift_distance,
            lift_speed,
            retract_speed,
            total_volume,
        ),
    ) = tuple((
        be_f64, be_f64, be_f64, be_f64, be_f64, be_u32, be_f64, be_f64, be_f64, be_f64,
    ))(input)?;
    let (input, thumbnail) = parse_photons_thumbnail(input)?;
    let (input, num_layers) = be_u32(input)?;
    let (input, layers) = nom::multi::count(parse_photons_layer, num_layers as usize)(input)?;
    Ok((
        input,
        PhotonsFile {
            pixelsize,
            layerheight,
            exposure_time,
            off_time,
            bottom_exposure_time,
            num_bottom_layers,
            lift_distance,
            lift_speed,
            retract_speed,
            total_volume,
            thumbnail,
            layers,
        },
    ))
}

fn main() -> std::io::Result<()> {
    let mut file = File::open("lowres.photons")?;
    let mut contents = Vec::new();
    let len = file.read_to_end(&mut contents)?;
    println!("Length: {}", len);
    let result = parse_photons_file(&contents);
    if let Ok((remaining, result)) = result {
        println!("Bytes remaining: {}", remaining.len());
        result.thumbnail.save("thumbnail.png").unwrap();
        println!("Num layers: {:?}", result.layers.len());
    } else {
        println!("Some kind of error parsing :(");
    }

    Ok(())
}

#[test]
fn test_rle_bits() {
    let data = RLEBits {
        num_ones: 9,
        data: vec![
            0xa2, 0x81, 0xb0, 0x81, 0x70, 0x81, 0x70, 0x81, 0xba, 0x01, 0x01,
        ],
    };
    let bits: Vec<bool> = data.iter().take(16 * 16).collect();
    #[rustfmt::skip]
    assert_eq!(bits, vec![
               false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false,
               false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false,
               false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false,
               false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false,
               false, false, false, false, false, false, true, true, false, false, false, false, false, false, false, false,
               false, false, false, false, false, false, true, true, false, false, false, false, false, false, false, false,
               false, false, false, false, false, false, false, true, true, false, false, false, false, false, false, false,
               false, false, false, false, false, false, false, false, true, true, false, false, false, false, false, false,
               false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false,
               false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false,
               false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false,
               false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false,
               false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false,
               false, false, false, false, false, false, false, false, true, false, false, false, false, false, false, false,
               false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false,
               false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false
    ]);
}

use crate::formats::photons::data::*;
use crate::parse_rgb565::parse_rgb565_image;
use image::RgbImage;
use nom::{number::complete::*, sequence::tuple, IResult};

fn parse_photons_thumbnail(input: &[u8]) -> IResult<&[u8], RgbImage> {
    let (input, (width, unk1, height, unk2)) = tuple((be_u32, be_u32, be_u32, be_u32))(input)?;
    if unk1 != 42 || unk2 != 10 {
        println!("Unknowns in thumbnail unexpected: {} {}", unk1, unk2);
        return Err(nom::Err::Failure((input, nom::error::ErrorKind::Verify)));
    }
    parse_rgb565_image(width, height, input)
}

fn parse_photons_layer(input: &[u8]) -> IResult<&[u8], PhotonsLayer> {
    let (input, (num_white, unknown1, width, height, total_size, width_revbits, height_revbits)) =
        tuple((be_u32, be_u64, be_u32, be_u32, be_u32, le_u16, le_u16))(input)?;
    if unknown1 != 0 {
        println!("Unknowns in layer unexpected: {:X}", unknown1);
        return Err(nom::Err::Failure((input, nom::error::ErrorKind::Verify)));
    }
    if width_revbits.reverse_bits() as u32 != width
        || height_revbits.reverse_bits() as u32 != height
    {
        println!(
            "Reverse-bit fields invalid: {:X} != {:X} or {:X} != {:X}",
            width,
            width_revbits.reverse_bits(),
            height,
            height_revbits.reverse_bits()
        );
        return Err(nom::Err::Failure((input, nom::error::ErrorKind::Verify)));
    }
    let (input, data) = nom::bytes::complete::take((total_size - 32) / 8)(input)?;
    let data = CompressedBitstream::new(data.to_vec(), num_white as usize);
    Ok((
        input,
        PhotonsLayer {
            width,
            height,
            data,
        },
    ))
}

pub fn parse_photons_file(input: &[u8]) -> IResult<&[u8], PhotonsFile> {
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

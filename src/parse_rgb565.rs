use image::{ImageBuffer, Rgb, RgbImage};
use nom::{number::complete::*, IResult};

fn upscale_5bit_to_8bit(input: u8) -> u8 {
    (input << 3) | (input >> 2)
}

fn upscale_6bit_to_8bit(input: u8) -> u8 {
    (input << 2) | (input >> 4)
}

fn parse_rgb565_pixel(input: &[u8]) -> IResult<&[u8], Rgb<u8>> {
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

pub fn parse_rgb565_image(width: u32, height: u32, input: &[u8]) -> IResult<&[u8], RgbImage> {
    let (input, pixels) = nom::multi::count(parse_rgb565_pixel, (width * height) as usize)(input)?;
    let pixels: Vec<u8> = pixels
        .iter()
        .flat_map(|p: &Rgb<u8>| p.0.iter())
        .cloned()
        .collect();
    Ok((input, ImageBuffer::from_vec(width, height, pixels).unwrap()))
}

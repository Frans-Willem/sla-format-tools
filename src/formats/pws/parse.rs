use crate::formats::pws::data::*;
use crate::parse_rgb565::parse_rgb565_image;
use image::RgbImage;
use nom::bytes::complete::tag;
use nom::{number::complete::*, sequence::tuple, IResult};

pub fn parse_pws_header(input: &[u8]) -> IResult<&[u8], PwsHeader> {
    let (input, (_, header_length)) = tuple((tag("HEADER\0\0\0\0\0\0"), le_u32))(input)?;
    if header_length != 80 {
        println!("Unexpected header length: {}", header_length);
        return Err(nom::Err::Failure((input, nom::error::ErrorKind::Verify)));
    }
    let (
        input,
        (
            pixel_size,
            layer_height,
            exposure_time,
            off_time,
            bottom_exposure_time,
            num_bottom_layers,
            lift_distance,
            lift_speed,
            drop_speed,
            volume,
            bits_per_pixel,
            width,
            height,
            weight,
            price,
            resin_type,
            use_individual_parameters,
            reserved,
        ),
    ) = tuple((
        le_f32,
        le_f32,
        le_f32,
        le_f32,
        le_f32,
        le_f32,
        le_f32,
        le_f32,
        le_f32,
        le_f32,
        le_u32,
        le_u32,
        le_u32,
        le_f32,
        le_f32,
        le_u32,
        le_u32,
        nom::bytes::complete::take(12 as usize),
    ))(input)?;
    if !reserved.iter().all(|b| *b == 0) {
        println!("Header reserved not empty {:?}", reserved);
        return Err(nom::Err::Failure((input, nom::error::ErrorKind::Verify)));
    }
    if use_individual_parameters > 1 {
        println!(
            "Expected 0 or 1 for boolean {:?}",
            use_individual_parameters
        );
        return Err(nom::Err::Failure((input, nom::error::ErrorKind::Verify)));
    }
    Ok((
        input,
        PwsHeader {
            pixel_size,
            layer_height,
            exposure_time,
            off_time,
            bottom_exposure_time,
            num_bottom_layers,
            lift_distance,
            lift_speed,
            drop_speed,
            volume,
            bits_per_pixel,
            width,
            height,
            weight,
            price,
            resin_type,
            use_individual_parameters: use_individual_parameters != 0,
        },
    ))
}

fn parse_pws_preview(input: &[u8]) -> IResult<&[u8], RgbImage> {
    let (input, (_, length, width, _, height)) = tuple((
        tag("PREVIEW\0\0\0\0\0"),
        le_u32,
        le_u32,
        tag("*\0\0\0"),
        le_u32,
    ))(input)?;
    if length != (width * height * 2) + 12 {
        println!(
            "Unexpected preview length: {} != 12 + {} * {} * 2",
            length, width, height
        );
        return Err(nom::Err::Failure((input, nom::error::ErrorKind::Verify)));
    }
    parse_rgb565_image(width, height, input)
}

struct LayerDef {
    offset: u32,
    length: u32,
    lift_distance: f32,
    lift_speed: f32,
    exposure_time: f32,
    layer_height: f32,
}

fn parse_pws_layerdef(input: &[u8]) -> IResult<&[u8], LayerDef> {
    let (input, (offset, length, lift_distance, lift_speed, exposure_time, layer_height, reserved)) =
        tuple((
            le_u32,
            le_u32,
            le_f32,
            le_f32,
            le_f32,
            le_f32,
            nom::bytes::complete::take(8 as usize),
        ))(input)?;
    if !reserved.iter().all(|v| *v == 0) {
        println!("Reserved fields not empty in layerdef {:?}", reserved);
        return Err(nom::Err::Failure((input, nom::error::ErrorKind::Verify)));
    }
    Ok((
        input,
        LayerDef {
            offset,
            length,
            lift_distance,
            lift_speed,
            exposure_time,
            layer_height,
        },
    ))
}

fn parse_pws_layerdefs(input: &[u8]) -> IResult<&[u8], Vec<LayerDef>> {
    let (input, (_, length, count)) = tuple((tag("LAYERDEF\0\0\0\0"), le_u32, le_u32))(input)?;
    if length != 4 + (count * 32) {
        println!(
            "Length of LayerDefs does not match: {} != 4 + {} * 32",
            length, count
        );
        return Err(nom::Err::Failure((input, nom::error::ErrorKind::Verify)));
    }
    nom::multi::count(parse_pws_layerdef, count as usize)(input)
}

pub fn parse_pws_file(input: &[u8]) -> IResult<&[u8], PwsFile> {
    let (
        mut rest,
        (
            _,
            version,
            area,
            header_addr,
            reserved1,
            preview_addr,
            reserved2,
            layerdef_addr,
            reserved3,
            _layers_addr,
        ),
    ) = tuple((
        tag("ANYCUBIC\0\0\0\0"),
        le_u32,
        le_u32,
        le_u32,
        le_u32,
        le_u32,
        le_u32,
        le_u32,
        le_u32,
        le_u32,
    ))(input)?;
    if reserved1 != 0 || reserved2 != 0 || reserved3 != 0 {
        println!(
            "One of the reserved values is not zero {} {} {}",
            reserved1, reserved2, reserved3
        );
        return Err(nom::Err::Failure((rest, nom::error::ErrorKind::Verify)));
    }
    if version != 1 || area != 4 {
        println!("Unexpected version or area: {} {}", version, area);
        return Err(nom::Err::Failure((rest, nom::error::ErrorKind::Verify)));
    }
    let (header_rest, header) = parse_pws_header(&input[header_addr as usize..])?;
    if header_rest.len() < rest.len() {
        rest = header_rest;
    }
    let (preview_rest, preview) = parse_pws_preview(&input[preview_addr as usize..])?;
    if preview_rest.len() < rest.len() {
        rest = preview_rest;
    }
    let (layerdefs_rest, layerdefs) = parse_pws_layerdefs(&input[layerdef_addr as usize..])?;
    if layerdefs_rest.len() < rest.len() {
        rest = layerdefs_rest;
    }

    let mut layers: Vec<PwsLayer> = Vec::new();
    for layerdef in layerdefs {
        let data =
            &input[layerdef.offset as usize..(layerdef.offset as usize + layerdef.length as usize)];
        let layer_rest = &input[(layerdef.offset as usize + layerdef.length as usize)..];
        if layer_rest.len() < rest.len() {
            rest = layer_rest
        }
        layers.push(PwsLayer {
            lift_distance: layerdef.lift_distance,
            lift_speed: layerdef.lift_speed,
            exposure_time: layerdef.exposure_time,
            layer_height: layerdef.layer_height,
            data: CompressedBitstream(data.into()),
        });
    }
    Ok((
        rest,
        PwsFile {
            header,
            preview,
            layers,
        },
    ))
}

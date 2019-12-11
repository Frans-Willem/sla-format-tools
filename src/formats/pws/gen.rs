use crate::formats::pws::data::*;
use cookie_factory::bytes::*;
use cookie_factory::combinator::*;
use cookie_factory::sequence::*;
use cookie_factory::multi::*;
use cookie_factory::{GenError, GenResult, SerializeFn, WriteContext};
use std::io::{Seek, SeekFrom, Write};
use image::{RgbImage, Rgb, Pixel};

fn store_pos64<W: Write + Seek, S: SerializeFn<W>, F: Fn(u64) -> S>(
    f: F,
    w: WriteContext<W>,
) -> Result<(WriteContext<W>, impl SerializeFn<W>), GenError> {
    let mut w = w;
    let pos = w.write.seek(SeekFrom::Current(0))?;
    let w = f(0xDEADBEEF)(w)?;
    let lam = move |w: WriteContext<W>| {
        let mut w = w;
        let saved_position = w.position;
        let old_pos = w.write.seek(SeekFrom::Current(0))?;
        let new_pos = w.write.seek(SeekFrom::Start(pos))?;
        let mut w = f(w.position)(w)?;
        w.write.seek(SeekFrom::Start(old_pos))?;
        w.position = saved_position;
        Ok(w)
    };
    Ok((w, lam))
}
fn store_pos32<W: Write + Seek, S: SerializeFn<W>, F: Fn(u32) -> S>(
    f: F,
    w: WriteContext<W>,
) -> Result<(WriteContext<W>, impl SerializeFn<W>), GenError> {
    store_pos64(move |x| f(x as u32), w)
}



const PWS_FILE_HEADER_SIZE : u32 = 0x30;
pub fn gen_pws_file<W: Write + 'static>(file: &PwsFile) -> impl SerializeFn<W> + '_ {
    let header_offset = PWS_FILE_HEADER_SIZE;
    let preview_offset = header_offset + PWS_HEADER_SIZE;
    let preview_size = calc_pws_preview_size(&file.preview);
    let layerdef_offset = preview_offset + preview_size;
    let layers_offset = layerdef_offset + calc_pws_layerdefs_size(&file.layers);

    tuple((
            slice(&b"ANYCUBIC\0\0\0\0"[..]),
            le_u32(1), // Version
            le_u32(4), // Area
            le_u32(header_offset),
            le_u32(0),
            le_u32(preview_offset),
            le_u32(0),
            le_u32(layerdef_offset),
            le_u32(0),
            le_u32(layers_offset),
            gen_pws_header(&file.header),
            gen_pws_preview(&file.preview),
            gen_pws_layerdefs(&file.layers, layers_offset),
            gen_pws_layers(&file.layers),
            ))
}

const PWS_HEADER_SIZE : u32 = 0x60;
fn gen_pws_header<W : Write>(header: &PwsHeader) -> impl SerializeFn<W> {
    tuple((
            tuple((
    slice(&b"HEADER\0\0\0\0\0\0"[..]),
    le_u32(80),
    le_f32(header.pixel_size),
    le_f32(header.layer_height),
    le_f32(header.exposure_time),
    )),
    tuple((
    le_f32(header.off_time),
    le_f32(header.bottom_exposure_time),
    le_f32(header.num_bottom_layers),
    le_f32(header.lift_distance),
    le_f32(header.lift_speed),
    le_f32(header.drop_speed),
    le_f32(header.volume),
    le_u32(header.bits_per_pixel),
    le_u32(header.width),
    le_u32(header.height),
    le_f32(header.weight),
    le_f32(header.price),
    le_u32(header.resin_type),
    le_u32(if header.use_individual_parameters {1} else{0}),
    le_u32(0),
    le_u32(0),
    le_u32(0),
    ))
    ))
}

fn encode_rgb565(pixel: &Rgb<u8>) -> u16 {
    let data = pixel.channels();
    let r = (data[0] >> 3) as u16;
    let g = (data[1] >> 2) as u16;
    let b = (data[2] >> 3) as u16;
    (b << 11) | (g << 5) | r
}

fn calc_pws_preview_size(preview: &RgbImage) -> u32 {
    16 + 12 + (preview.width() * preview.height() * 2)
}

fn gen_pws_preview<W : Write>(preview: &RgbImage) -> impl SerializeFn<W> {
    let gen_pixels : Vec<_> = preview.pixels().map(encode_rgb565).collect();
    tuple((
            slice(&b"PREVIEW\0\0\0\0\0"[..]),
            le_u32((preview.width()*preview.height()*2) + 12),
            le_u32(preview.width()),
            slice(&b"*\0\0\0"[..]),
            le_u32(preview.height()),
            many_ref(gen_pixels, le_u16),
            ))
}

const PWS_LAYERDEF_SIZE : u32 = 32;
fn calc_pws_layerdefs_size(layers: &[PwsLayer]) -> u32 {
    16 + 4 + (PWS_LAYERDEF_SIZE * layers.len() as u32)
}

fn gen_pws_layerdef<W: Write>(layer: (&PwsLayer, u32)) -> impl SerializeFn<W> {
    tuple((
            le_u32(layer.1),
            le_u32(layer.0.data.0.len() as u32),
            le_f32(layer.0.lift_distance),
            le_f32(layer.0.lift_speed),
            le_f32(layer.0.exposure_time),
            le_f32(layer.0.layer_height),
            slice(&b"\0\0\0\0\0\0\0\0"[..]),
            ))
}
fn gen_pws_layerdefs<W: Write + 'static>(layers: &[PwsLayer], layers_offset: u32) -> impl SerializeFn<W> + '_ {
    let x : Vec<(&PwsLayer, u32)> = layers.iter().scan(layers_offset, move |offset, layer| {
        let current_offset : u32 = *offset;
        *offset = current_offset + layer.data.0.len() as u32;
        Some((layer, current_offset))
    }).collect();
    tuple((
            slice(&b"LAYERDEF\0\0\0\0"[..]),
            le_u32(4 + PWS_LAYERDEF_SIZE * layers.len() as u32),
            le_u32(layers.len() as u32),
            many_ref(x, gen_pws_layerdef)
            ))
}

fn gen_pws_layer<W: Write + 'static>(layer: &PwsLayer) -> impl SerializeFn<W> + '_ {
    slice(&layer.data.0)
}

fn gen_pws_layers<W: Write + 'static>(layers: &[PwsLayer]) -> impl SerializeFn<W> + '_ {
    many_ref(layers, gen_pws_layer)
}

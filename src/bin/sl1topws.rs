use image::{Rgb, RgbImage};
use ini;
use std::fs::File;
use std::io::Read;
use zip;
use SLAFormatConverter::formats::pws;
//use rust_ini::ini;

fn main() -> std::io::Result<()> {
    let f = File::open("example_files/ArnoldOrczenegger_v2.sl1").unwrap();
    let mut z = zip::read::ZipArchive::new(f).unwrap();
    let config = ini::Ini::read_from(&mut z.by_name("config.ini").unwrap()).unwrap();

    println!("Properties:");
    for (key, value) in config.general_section().iter() {
        println!("{} => {}", key, value)
    }

    let num_slow = config.general_section()["numSlow"].parse::<u32>().unwrap();
    let num_fast = config.general_section()["numFast"].parse::<u32>().unwrap();
    let num_fade = config.general_section()["numFade"].parse::<u32>().unwrap();
    let num_total = num_slow + num_fast;
    println!(
        "Num layers: {}, Slow: {}, Fade: {}",
        num_total, num_slow, num_fade
    );
    let exposure_time = config.general_section()["expTime"].parse::<f32>().unwrap();
    let exposure_time_first = config.general_section()["expTimeFirst"]
        .parse::<f32>()
        .unwrap();
    let layer_height = config.general_section()["layerHeight"]
        .parse::<f32>()
        .unwrap();

    let header = pws::data::PwsHeader {
        pixel_size: 47.25, //0.04725,
        layer_height,
        exposure_time,
        off_time: 1.0,
        bottom_exposure_time: exposure_time_first,
        num_bottom_layers: (num_slow + num_fade) as f32,
        lift_distance: 6.0,
        lift_speed: 3.0,
        drop_speed: 3.0,
        volume: 0.0,
        bits_per_pixel: 4,
        width: 1440,
        height: 2560,
        weight: 0.0,
        price: 0.0,
        resin_type: 36,
        use_individual_parameters: true,
    };
    let preview = RgbImage::from_pixel(244, 168, Rgb([0, 0, 0]));
    let mut layers: Vec<pws::data::PwsLayer> = Vec::new();
    for index in 0..num_total {
        println!("Layer {}...", index);
        let mut input_image: Vec<u8> = Vec::new();
        z.by_name(&format!(
            "{}{:05}.png",
            config.general_section()["jobDir"],
            index
        ))
        .unwrap()
        .read_to_end(&mut input_image)
        .unwrap();
        let input_image =
            image::load_from_memory_with_format(&input_image, image::ImageFormat::PNG)
                .unwrap()
                .to_luma();
        if input_image.width() != header.width || input_image.height() != header.height {
            panic!("Image size mismatch");
        }
        let mut compressed = pws::data::CompressedBitstream(Vec::new());
        let bpp = header.bits_per_pixel as usize;
        for bit in 0..bpp {
            let threshold = (bit * 256) / bpp;
            let threshold = threshold + (256 / (bpp * 2));
            let bitstream: Vec<bool> = input_image
                .pixels()
                .map(move |p| p.0[0] as usize >= threshold)
                .collect();
            let pws::data::CompressedBitstream(mut sublayer_compressed) =
                pws::data::CompressedBitstream::compress(
                    &bitstream,
                    (header.width * header.height) as usize,
                );
            compressed.0.append(&mut sublayer_compressed);
        }
        let exposure_time = if index < num_slow {
            header.bottom_exposure_time
        } else if index < (num_slow + num_fade) {
            let fade: f32 = (index - num_slow) as f32 / num_fade as f32;
            (header.exposure_time * fade) + (header.bottom_exposure_time * (1.0 - fade))
        } else {
            header.exposure_time
        };
        let layer = pws::data::PwsLayer {
            lift_distance: header.lift_distance,
            lift_speed: header.lift_speed,
            exposure_time,
            layer_height: header.layer_height,
            data: compressed,
        };
        layers.push(layer);
    }
    println!("{:?}", header);
    let pws_file = pws::data::PwsFile {
        header,
        preview,
        layers,
    };
    let f = File::create("converted.pws").unwrap();
    let f = cookie_factory::gen(pws::gen::gen_pws_file(&pws_file), f).unwrap();
    Ok(())
}

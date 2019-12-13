use clap::{crate_version, App, Arg};
use image::{Rgb, RgbImage, GrayImage};
use ini;
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use sla_format_tools::formats::pws;
use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use std::sync::{Arc, Mutex};
use zip;

fn check_antialias_arg(input: String) -> Result<(), String> {
    match input.as_ref() {
        "1" | "2" | "4" | "8" => Ok(()),
        _ => Err("Invalid anti-alias option, allowed values are 1, 2, 4, or 8".to_string()),
    }
}

fn iterate_sl1_layers(sl1file: zip::read::ZipArchive<File>, job_dir: String, num_layers: usize) -> impl IndexedParallelIterator<Item = GrayImage> {
    let sl1file = Mutex::new(sl1file);
    let layer_indices = (0..num_layers).into_par_iter();
    // Reading into Vec<u8>, with lock on ZipArchive
    let layer_image_files = layer_indices.map(move |index| {
        let mut sl1file = sl1file.lock().unwrap();
        let mut image_bin = Vec::new();
        sl1file
            .by_name(&format!("{}{:05}.png", job_dir, index))
            .unwrap()
            .read_to_end(&mut image_bin)
            .unwrap();
        image_bin
    });
    // Decoding png file
    layer_image_files.map(|image_bin| {
        image::load_from_memory_with_format(&image_bin, image::ImageFormat::PNG)
            .unwrap()
            .to_luma()
    })
}

fn convert_sl1_layers(
    sl1file: zip::read::ZipArchive<File>,
    job_dir: String,
    num_layers: usize,
    bits_per_pixel: usize,
    layer_height: f32,
    lift_distance: f32,
    lift_speed: f32,
    exposure_time: f32,
    bottom_exposure_time: f32,
    num_slow: usize,
    num_fade: usize,
) -> (HashSet<(u32, u32)>, Vec<pws::data::PwsLayer>) {
    let layer_images = iterate_sl1_layers(sl1file, job_dir, num_layers);
    let layer_compressed = layer_images.enumerate().map(|(index, image)| {
        println!("Compressing layer {}", index);
        let thresholds = (0..bits_per_pixel).map(|bit| {
            let threshold = (bit * 256) / bits_per_pixel;
            let threshold = threshold + (256 / (bits_per_pixel * 2));
            threshold
        });
        let mut compressed = pws::data::CompressedBitstream::new();
        for threshold in thresholds {
            let bitstream = image.pixels().map(|p| p.0[0] as usize >= threshold);
            compressed.compress_append(bitstream);
        }
        let exposure_time = if index < num_slow {
            bottom_exposure_time
        } else if index < (num_slow + num_fade) {
            let fade: f32 = (index - num_slow) as f32 / num_fade as f32;
            (exposure_time * fade) + (bottom_exposure_time * (1.0 - fade))
        } else {
            exposure_time
        };
        (
            (image.width(), image.height()),
            pws::data::PwsLayer {
                lift_distance,
                lift_speed,
                exposure_time,
                layer_height,
                data: compressed,
            },
        )
    });
    let (layer_sizes, layer_compressed) = layer_compressed
        .unzip::<(u32, u32), pws::data::PwsLayer, HashSet<(u32, u32)>, Vec<pws::data::PwsLayer>>();
    (layer_sizes, layer_compressed)
}

fn main() {
    let args = App::new("SL1 to PWS converter")
        .version(crate_version!())
        .author("Frans-willem Hardijzer <fw@hardijzer.nl>")
        .about("Converts Prusa SL1 (.sl1) files to Anycubic Photon (S) (.pws) files")
        .arg(
            Arg::with_name("input")
                .short("i")
                .long("input")
                .value_name("FILE")
                .help("Input .sl1 file")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("output")
                .short("o")
                .long("output")
                .value_name("FILE")
                .help("Output .pws file")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("antialias")
                .short("a")
                .long("antialias")
                .value_name("AA")
                .default_value("1")
                .validator(check_antialias_arg)
                .help("Anti-aliasing levels"),
        )
        .get_matches();

    let input_fname = args.value_of("input").unwrap();
    let output_fname = args.value_of("output").unwrap();
    let bits_per_pixel = args.value_of("antialias").unwrap().parse::<u32>().unwrap();

    let mut z = zip::read::ZipArchive::new(File::open(input_fname).unwrap()).unwrap();
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
    let lift_distance = 6.0;
    let lift_speed = 1.5;
    let drop_speed = 3.0;

    let preview = RgbImage::from_pixel(224, 168, Rgb([0, 0, 0]));
    let (sizes, layers) = convert_sl1_layers(
        z,
        config.general_section()["jobDir"].clone(),
        num_total as usize,
        bits_per_pixel as usize,
        layer_height,
        lift_distance,
        lift_speed,
        exposure_time,
        exposure_time_first,
        num_slow as usize,
        num_fade as usize,
    );
    if sizes.len() != 1 {
        panic!("Sizes do not match between layers!");
    }
    let size = sizes.into_iter().next().unwrap();
    let header = pws::data::PwsHeader {
        pixel_size: 47.25, //0.04725,
        layer_height,
        exposure_time,
        off_time: 1.0,
        bottom_exposure_time: exposure_time_first,
        num_bottom_layers: (num_slow + num_fade) as f32,
        lift_distance,
        lift_speed,
        drop_speed,
        volume: 0.0,
        bits_per_pixel,
        width: size.0,
        height: size.1,
        weight: 0.0,
        price: 0.0,
        resin_type: 36,
        use_individual_parameters: true,
    };
    let pws_file = pws::data::PwsFile {
        header,
        preview,
        layers,
    };
    cookie_factory::gen(
        pws::gen::gen_pws_file(&pws_file),
        File::create(output_fname).unwrap(),
    )
    .unwrap();
}

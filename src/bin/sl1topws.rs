use clap::{crate_version, App, Arg};
use image::{GrayImage, Rgb, RgbImage};
use ini;
use pbr::ProgressBar;
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use sla_format_tools::formats::pws;
use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use std::sync::Mutex;
use zip;

fn check_antialias_arg(input: String) -> Result<(), String> {
    match input.as_ref() {
        "1" | "2" | "4" | "8" => Ok(()),
        _ => Err("Invalid anti-alias option, allowed values are 1, 2, 4, or 8".to_string()),
    }
}

fn check_parse_arg<T: std::str::FromStr>(input: String) -> Result<(), String>
where
    T::Err: std::string::ToString,
{
    input.parse::<T>().map(drop).map_err(|e| e.to_string())
}

fn iterate_sl1_layers(
    sl1file: zip::read::ZipArchive<File>,
    job_dir: String,
    num_layers: usize,
) -> impl IndexedParallelIterator<Item = GrayImage> {
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
    let pb = Mutex::new(ProgressBar::new(num_layers as u64));
    pb.lock().unwrap().message("Converting layers: ");
    let layer_compressed = layer_images.enumerate().map(|(index, image)| {
        let data = pws::data::CompressedBitstream::from_image(&image, bits_per_pixel);
        let exposure_time = if index < num_slow {
            bottom_exposure_time
        } else if index < (num_slow + num_fade) {
            let fade: f32 = (index - num_slow) as f32 / num_fade as f32;
            exposure_time.mul_add(fade, bottom_exposure_time * (1.0 - fade))
        } else {
            exposure_time
        };
        pb.lock().unwrap().inc();
        (
            (image.width(), image.height()),
            pws::data::PwsLayer {
                lift_distance,
                lift_speed,
                exposure_time,
                layer_height,
                data,
            },
        )
    });
    let (layer_sizes, layer_compressed) = layer_compressed
        .unzip::<(u32, u32), pws::data::PwsLayer, HashSet<(u32, u32)>, Vec<pws::data::PwsLayer>>();
    pb.lock().unwrap().finish_print("Done");
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
                .value_name("filename")
                .help("Input .sl1 file")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("output")
                .short("o")
                .long("output")
                .value_name("filename")
                .help("Output .pws file")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("antialias")
                .short("a")
                .long("antialias")
                .value_name("bpp")
                .default_value("4")
                .validator(check_antialias_arg)
                .help("Anti-aliasing levels"),
        )
        .arg(
            Arg::with_name("lift-distance")
                .short("l")
                .long("lift-distance")
                .value_name("mm")
                .default_value("6.0")
                .validator(check_parse_arg::<f32>)
                .help("Lift distance after layers in millimeter"),
        )
        .arg(
            Arg::with_name("lift-speed")
                .short("s")
                .long("lift-speed")
                .value_name("mm/sec")
                .default_value("1.5")
                .validator(check_parse_arg::<f32>)
                .help("Lift speed in millimeter per second"),
        )
        .arg(
            Arg::with_name("drop-speed")
                .short("d")
                .long("drop-speed")
                .value_name("mm/sec")
                .default_value("2.5")
                .validator(check_parse_arg::<f32>)
                .help("Drop speed in millimeter per second"),
        )
        .get_matches();

    let input_fname = args.value_of("input").unwrap();
    let output_fname = args.value_of("output").unwrap();
    let bits_per_pixel = args.value_of("antialias").unwrap().parse::<u32>().unwrap();

    let mut z = zip::read::ZipArchive::new(File::open(input_fname).unwrap()).unwrap();
    let config = ini::Ini::read_from(&mut z.by_name("config.ini").unwrap()).unwrap();

    let num_slow = config.general_section()["numSlow"].parse::<u32>().unwrap();
    let num_fast = config.general_section()["numFast"].parse::<u32>().unwrap();
    let num_fade = config.general_section()["numFade"].parse::<u32>().unwrap();
    let num_total = num_slow + num_fast;
    let exposure_time = config.general_section()["expTime"].parse::<f32>().unwrap();
    let exposure_time_first = config.general_section()["expTimeFirst"]
        .parse::<f32>()
        .unwrap();
    let layer_height = config.general_section()["layerHeight"]
        .parse::<f32>()
        .unwrap();

    let lift_distance = args
        .value_of("lift-distance")
        .unwrap()
        .parse::<f32>()
        .unwrap();
    let lift_speed = args.value_of("lift-speed").unwrap().parse::<f32>().unwrap();
    let drop_speed = args.value_of("drop-speed").unwrap().parse::<f32>().unwrap();

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

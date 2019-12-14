use pbr::ProgressBar;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use sla_format_tools::formats::pws;
use std::fs::File;
use std::io::Read;
use std::sync::Mutex;

fn main() {
    let input_fname = "example_files/ArnoldOrczenegger_v2.pws";
    let mut input = Vec::new();
    File::open(input_fname)
        .unwrap()
        .read_to_end(&mut input)
        .unwrap();
    let (remaining_input, pws_file) = pws::parse::parse_pws_file(&input).unwrap();
    assert_eq!(remaining_input.len(), 0);
    let mut pb = ProgressBar::new(pws_file.layers.len() as u64);
    pb.message("Verifying layer compression: ");
    let pb = Mutex::new(pb);
    let layers_verified = pws_file
        .layers
        .par_iter()
        .enumerate()
        .map(|(index, layer)| {
            let uncompressed = layer
                .data
                .to_image(pws_file.header.width, pws_file.header.height)
                .unwrap();
            let recompressed = pws::data::CompressedBitstream::from_image(
                &uncompressed,
                pws_file.header.bits_per_pixel as usize,
            );
            let ret = recompressed == layer.data;
            if !ret {
                println!("Recompression did not yield same result on layer {}", index);
            }
            pb.lock().unwrap().inc();
            ret
        })
        .all(std::convert::identity);
    pb.lock().unwrap().finish_print("Done");
    assert_eq!(layers_verified, true);
    let (output, _output_size) =
        cookie_factory::gen(pws::gen::gen_pws_file(&pws_file), Vec::new()).unwrap();
    assert_eq!(input, output);
    println!("Reading PWS & re-writing it yielded same file, success.");
}

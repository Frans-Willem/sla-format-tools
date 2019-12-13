use image::RgbImage;

#[derive(Debug)]
pub struct PwsHeader {
    pub pixel_size: f32,           // in um
    pub layer_height: f32,         // in mm
    pub exposure_time: f32,        // in sec
    pub off_time: f32,             // in sec
    pub bottom_exposure_time: f32, // in sec
    pub num_bottom_layers: f32,    // count, why on earth a float?!
    pub lift_distance: f32,        // in mm
    pub lift_speed: f32,           // in mm/sec
    pub drop_speed: f32,           // in mm/sec
    pub volume: f32,               // ?
    pub bits_per_pixel: u32,
    pub width: u32,
    pub height: u32,
    pub weight: f32,
    pub price: f32, // in USD (resin cost)
    pub resin_type: u32,
    pub use_individual_parameters: bool,
}

#[derive(PartialEq, Debug)]
pub struct CompressedBitstream(pub Vec<u8>);

pub struct PwsLayer {
    pub lift_distance: f32,
    pub lift_speed: f32,
    pub exposure_time: f32,
    pub layer_height: f32,
    pub data: CompressedBitstream,
}

pub struct PwsFile {
    pub header: PwsHeader,
    pub preview: RgbImage,
    pub layers: Vec<PwsLayer>,
}

impl CompressedBitstream {
    pub fn new() -> CompressedBitstream {
        CompressedBitstream(Vec::new())
    }

    pub fn decompress(&self) -> Vec<bool> {
        let mut output = Vec::new();
        for v in self.0.iter() {
            let value = (*v & 0x80) != 0;
            let mut repeat = (*v & 0x7F);
            while repeat > 0 {
                repeat -= 1;
                output.push(value);
            }
        }
        output
    }

    pub fn compress<B : Iterator<Item = bool>>(bitstream: B) -> CompressedBitstream {
        let mut x = CompressedBitstream::new();
        x.compress_append(bitstream);
        x
    }

    pub fn compress_append<B : Iterator<Item = bool>>(&mut self, bitstream: B) {
        let mut previous_value = false;
        let mut previous_count = 0;
        let mut bitstream = bitstream;
        while let Some(value) = bitstream.next() {
            if previous_value != value {
                if previous_count > 0 {
                    self.0.push(previous_count | if previous_value { 0x80 } else { 0});
                }
                previous_value = value;
                previous_count = 1;
            } else {
                if previous_count > 125 {
                    self.0.push(125 | if previous_value { 0x80 } else { 0});
                    previous_count -= 125;
                }
                previous_count += 1;
            }
        }
        if previous_count > 0 {
            self.0.push(previous_count | if previous_value { 0x80 } else { 0});
        }
    }
}

#[test]
fn test_decompress_compress() {
    // These tests check if the last byte is allowed to repeat 126 times, whereas all others only
    // 125 times (no idea why anycubic does this).
    let input = CompressedBitstream(vec![0x80 | 126]);
    let recompressed = CompressedBitstream::compress(input.decompress().into_iter());
    assert_eq!(input, recompressed);
    let input = CompressedBitstream(vec![0x80 | 125, 125, 126]);
    let recompressed = CompressedBitstream::compress(input.decompress().into_iter());
    assert_eq!(input, recompressed);

}

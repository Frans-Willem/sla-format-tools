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

#[derive(PartialEq)]
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

    pub fn compress(bitstream: &[bool], boundary: usize) -> CompressedBitstream {
        let mut index: usize = 0;
        let mut data: Vec<u8> = Vec::new();
        while index < bitstream.len() {
            let value = bitstream[index];
            index += 1;
            let mut repeat: u8 = 1;
            while index < bitstream.len()
                && repeat < 125
                && bitstream[index] == value
                && index % boundary != 0
            {
                repeat += 1;
                index += 1;
            }
            // Don't ask. Only such that recompressing Anycubic Photon Workshop files end up with
            // identical files.
            if repeat == 125
                && index < bitstream.len()
                && bitstream[index] == value
                && (index + 1) % boundary == 0
            {
                repeat += 1;
                index += 1;
            }
            data.push(repeat | if value { 0x80 } else { 0 });
        }
        CompressedBitstream(data)
    }
}

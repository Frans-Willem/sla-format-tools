pub struct PhotonsFile {
    pub pixelsize: f64,            // XY size of pixel, in mm
    pub layerheight: f64,          // Layer height, in mm
    pub exposure_time: f64,        // Normal exposure time, in seconds
    pub off_time: f64,             // Off-time, in seconds
    pub bottom_exposure_time: f64, // Bottom exposure time, in seconds
    pub num_bottom_layers: u32,    // Number of bottom layers
    pub lift_distance: f64,        // in mm
    pub lift_speed: f64,           // in mm/sec
    pub retract_speed: f64,        // in mm/sec
    pub total_volume: f64,         // in mm^3
    pub thumbnail: image::RgbImage,
    pub layers: Vec<PhotonsLayer>,
}

pub struct PhotonsLayer {
    pub width: u32,
    pub height: u32,
    pub data: CompressedBitstream,
}

#[derive(PartialEq)]
pub struct CompressedBitstream {
    pub num_ones: usize,
    pub data: Vec<u8>,
}

impl CompressedBitstream {
    pub fn new(data: Vec<u8>, num_ones: usize) -> CompressedBitstream {
        CompressedBitstream { num_ones, data }
    }
    pub fn decompress(&self, len: usize) -> Vec<bool> {
        let mut ret = vec![false; len];
        let mut ones_remaining = self.num_ones;
        let mut index: usize = 0;
        for b in self.data.iter() {
            let b = b.reverse_bits();
            let value = (b & 0x80) != 0;
            let mut repeat: usize = ((b & 0x7F) as usize) + 1;
            if value {
                if repeat > ones_remaining {
                    repeat = ones_remaining;
                }
                ones_remaining -= repeat;
            }
            while repeat > 0 && index < ret.len() {
                ret[index] = value;
                repeat -= 1;
                index += 1;
            }
        }
        ret
    }
    pub fn compress(bitstream: &[bool]) -> CompressedBitstream {
        /*
         * This implementation is horrible, but it took some convincing to get exactly the same
         * outputs as ChiTuBox. (e.g. the parts after the loop).
         * I suspect another implementation may be faster & better (e.g. for empty images, num_ones
         * set to 0, and a single 1 should suffice, but I haven't tested this)
         */
        let mut data = Vec::new();
        let mut num_ones: usize = 0;
        let mut last_value = false;
        let mut last_value_count: u8 = 0;
        for b in bitstream.iter() {
            let b = *b;
            if b {
                num_ones += 1;
            }
            if b != last_value {
                if last_value_count > 0 {
                    let encoded: u8 = (last_value_count - 1) | (if last_value { 0x80 } else { 0 });
                    data.push(encoded.reverse_bits());
                }
                last_value = b;
                last_value_count = 1;
            } else {
                last_value_count += 1;
                if last_value_count == 128 {
                    let encoded: u8 = (last_value_count - 1) | (if last_value { 0x80 } else { 0 });
                    data.push(encoded.reverse_bits());
                    last_value_count = 0;
                }
            }
        }
        if last_value_count > 0 {
            last_value_count += 1;
            let encoded: u8 = (last_value_count - 1) | (if last_value { 0x80 } else { 0 });
            data.push(encoded.reverse_bits());
        } else if let Some(last) = data.pop() {
            if last == 0xfe {
                data.push(0x01);
            } else {
                data.push(last);
            }
        }
        CompressedBitstream { num_ones, data }
    }

    pub fn debug_out<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        writeln!(w, "Num ones: {}", self.num_ones)?;
        for b in self.data.iter() {
            let b = b.reverse_bits();
            let value = (b & 0x80) != 0;
            let repeat = b & 0x7F;
            writeln!(w, "{} repeat {}", value, repeat + 1)?;
        }
        Ok(())
    }
}

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use png::{Decoder, Encoder};
use clap::{Arg, App};

fn main() -> Result<(), String> {
    // CLI interface
    let matches = App::new("Xor Image Encoder")
        .version("1.0")
        .author("Jonas Maier <1.jmaier.3@gmail.com>")
        .about("Takes a black and white image and outputs two images that when overlayed display the original image\nOnly supports png images because I am lazy")
        .arg(Arg::with_name("image")
            .short("i")
            .long("image")
            .value_name("IMAGE.PNG")
            .help("Specifies image input")
            .required(true)
            .takes_value(true))
        .arg(Arg::with_name("cutoff")
            .short("c")
            .long("cutoff")
            .value_name("0-255")
            .help("Specifies cutoff: everything below is black, everything above is white")
            .takes_value(true))
        .arg(Arg::with_name("output")
            .short("o")
            .long("output")
            .value_name("OUT.PNG")
            .help("Specifies image output (outputs to OUT1.PNG and OUT2.PNG)")
            .takes_value(true))
        .get_matches();


    // input parsing
    let image_file = matches.value_of("image").ok_or_else(|| "No image file given".to_string())?;

    let gray_cutoff = match matches.value_of("cutoff").unwrap_or("128").parse::<u64>() {
        Ok(val) => val,
        Err(_) => return Err("Invalid gray cutoff value".to_string()),
    };

    let output_file = match matches.value_of("output") {
        Some(output) => output,
        None => image_file,
    };

    let output_file = Path::new(output_file).file_stem().ok_or_else(|| "Could not extract file stem".to_string())?.to_os_string().into_string().map_err(|_| "Error processing image file name".to_string())?;


    // reading image
    let decoder = Decoder::new(File::open(image_file).map_err(|_| format!("Could not open file {}", image_file))?);
    let (info, mut reader) = decoder.read_info().map_err(|_| format!("Could not read image info of {}", image_file))?;
    let mut image = vec![0; info.buffer_size()];
    reader.next_frame(&mut image).map_err(|_| format!("Could not read image {}", image_file))?;

    let bytes_per_pixel = info.buffer_size() as u64 / info.width as u64 / info.height as u64;

    let apply_pattern = |img: &mut Vec<Vec<bool>>, x: usize, y: usize, pattern: bool| {
        img[2*x+0][2*y+0] =  pattern;
        img[2*x+1][2*y+0] = !pattern;
        img[2*x+0][2*y+1] = !pattern;
        img[2*x+1][2*y+1] =  pattern;
    };

    // create boolean image data of the two images
    let mut out = vec![vec![vec![false; 2 * info.height as usize]; 2 * info.width as usize]; 2];
    for x in 0..info.width as u64 {
        for y in 0..info.height as u64 {
            let white = {
                let mut sum: u64 = 0;
                let idx = y * info.width as u64 + x;
                let idx = bytes_per_pixel * idx;
                for i in idx..(idx+bytes_per_pixel) {
                    sum += image[i as usize] as u64;
                }
                sum /= bytes_per_pixel as u64;
                let variance = rand::random::<u16>() as i64 % 120 - 60;
                sum as i64 + variance >= gray_cutoff as i64
            };
            let pattern: bool = rand::random();
            apply_pattern(&mut out[0], x as _, y as _, pattern);
            apply_pattern(&mut out[1], x as _, y as _, white == pattern);
        }
    }

    let out: Vec<Vec<u8>> = out.into_iter().map(|img| {
        // flatmap the image
        let width = 2 * info.width as usize; let height = 2 * info.height as usize;
        let mut flat = vec![false; width * height];
        for x in 0..width {
            for y in 0..height {
                flat[y * width + x] = img[x][y];
            }
        }
        // convert bool to RGBA bytes
        flat.into_iter().map(|x| if x {vec![255u8, 255u8, 255u8, 0u8]} else {vec![0u8, 0u8, 0u8, 255u8]}).flat_map(|x| x.into_iter()).collect()
    }).collect();
    
    // write the two output images
    for i in 0..2 {
        let output_path = format!("{}{}.png", output_file, i+1);
        let file = File::create(&output_path).map_err(|_| format!("Could not write to {}", &output_path))?;
        let writer = BufWriter::new(file);
        let mut encoder = Encoder::new(writer, 2 * info.width, 2 * info.height);
        encoder.set_color(png::ColorType::RGBA);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|_| format!("Failed to write header for {}", &output_path))?;
        writer.write_image_data(&out[i]).map_err(|_| format!("Failed to write image data for {}", &output_path))?;
    }

    Ok(())
}

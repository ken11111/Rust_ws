use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

fn main() -> std::io::Result<()> {
    let input_file = "output.mjpeg";
    let output_dir = "frames";

    println!("Reading {}...", input_file);

    // Read entire file
    let mut file = File::open(input_file)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;

    println!("File size: {} bytes", data.len());

    // Create output directory
    std::fs::create_dir_all(output_dir)?;

    // Find all JPEG frames (SOI marker FF D8)
    let mut frame_count = 0;
    let mut i = 0;

    while i < data.len() - 1 {
        // Look for JPEG SOI marker (FF D8)
        if data[i] == 0xFF && data[i + 1] == 0xD8 {
            let start = i;

            // Find EOI marker (FF D9)
            let mut end = start + 2;
            while end < data.len() - 1 {
                if data[end] == 0xFF && data[end + 1] == 0xD9 {
                    end += 2; // Include EOI marker
                    break;
                }
                end += 1;
            }

            if end < data.len() {
                // Extract JPEG data
                let jpeg_data = &data[start..end];

                // Save to file
                let filename = format!("{}/frame_{:06}.jpg", output_dir, frame_count + 1);
                let mut out_file = File::create(&filename)?;
                out_file.write_all(jpeg_data)?;

                println!("Saved {} ({} bytes)", filename, jpeg_data.len());
                frame_count += 1;

                i = end;
            } else {
                break;
            }
        } else {
            i += 1;
        }
    }

    println!("\nExtracted {} frames to {}/", frame_count, output_dir);
    println!("View with: eog {}/ or feh {}/", output_dir, output_dir);

    Ok(())
}

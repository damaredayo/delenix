#[cfg(feature = "tesseract")]
mod ocr {
    use image::{EncodableLayout, ImageBuffer, ImageFormat, Luma, Pixel};
    use std::{collections::HashMap, io::Cursor};
    use tesseract::Tesseract;

    pub fn ocr(
        tessdata_path: &Option<String>,
        img: &[u8],
    ) -> Result<String, Box<dyn std::error::Error>> {
        let img_format = match image::guess_format(img) {
            Ok(f) => f,
            Err(_) => ImageFormat::Png, // default to png, if it dies it dies, we tried, you shouldnt have given an insane image format
        };

        let image = image::load(Cursor::new(img), img_format)?;

        let mut grayscale_image = image.into_luma8();

        let most_common_color = find_most_common_color(&grayscale_image);

        if most_common_color == Colour::Black {
            invert_colors(&mut grayscale_image);
        }

        let mut tiff_buffer = Cursor::new(Vec::new());

        grayscale_image
            .write_to(&mut tiff_buffer, ImageFormat::Tiff)
            .unwrap();

        let tess = Tesseract::new(tessdata_path.as_deref(), Some("eng"))?;

        let text = tess
            .set_image_from_mem(tiff_buffer.into_inner().as_bytes())?
            .set_source_resolution(600)
            .get_text()?;

        Ok(text)
    }

    #[derive(PartialEq, Debug)]
    enum Colour {
        Black,
        White,
    }

    // check if black or white is more common (we can pretty safely assume this would be the background) and invert the image if black is more common
    fn find_most_common_color(image_buffer: &ImageBuffer<Luma<u8>, Vec<u8>>) -> Colour {
        let threshold = 128;

        let mut color_counts: HashMap<u8, u32> = HashMap::new();

        for pixel in image_buffer.pixels() {
            let luminance = pixel[0];
            let color = if luminance < threshold { 0 } else { 255 };
            *color_counts.entry(color).or_insert(0) += 1;
        }

        let black_count = color_counts.get(&0).unwrap_or(&0);
        let white_count = color_counts.get(&255).unwrap_or(&0);

        if black_count > white_count {
            Colour::Black
        } else {
            Colour::White
        }
    }

    fn invert_colors(image_buffer: &mut ImageBuffer<Luma<u8>, Vec<u8>>) {
        for pixel in image_buffer.pixels_mut() {
            let luma = pixel.channels()[0];
            let inverted_luma = 255 - luma;
            *pixel = Luma([inverted_luma]);
        }
    }
}

#[cfg(not(feature = "tesseract"))]
mod ocr {
    pub fn ocr(_: &Option<String>, _: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
        tracing::error!(
            "Tesseract not enabled. Please rebuild with the `tesseract` feature to enable it."
        );
        Ok("".to_string())
    }
}

pub use ocr::*;

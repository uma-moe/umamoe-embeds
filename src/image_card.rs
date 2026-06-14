use std::io::Cursor;

use anyhow::Result;
use font8x8::{UnicodeFonts, BASIC_FONTS};
use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};

use crate::embed::EmbedMetadata;

const WIDTH: u32 = 1200;
const HEIGHT: u32 = 630;

type Color = Rgba<u8>;

pub fn render_png(meta: &EmbedMetadata) -> Result<Vec<u8>> {
    let accent = accent_for_kind(&meta.kind_label);
    let mut image = RgbaImage::new(WIDTH, HEIGHT);

    draw_background(&mut image, accent);
    fill_rect(&mut image, 0, 0, WIDTH as i32, 10, accent);
    fill_rect(&mut image, 68, 82, 6, 375, accent);

    draw_text(
        &mut image,
        "uma.moe",
        910,
        64,
        3,
        Rgba([126, 216, 169, 255]),
    );
    draw_pill(&mut image, 78, 58, &meta.kind_label, accent);

    let title_lines = wrap_text(&meta.title, 31);
    let mut y = 128;
    for line in title_lines.iter().take(3) {
        draw_text(&mut image, line, 96, y, 4, Rgba([245, 248, 250, 255]));
        y += 50;
    }

    y += 18;
    for line in wrap_text(&meta.description, 58).iter().take(3) {
        draw_text(&mut image, line, 98, y, 2, Rgba([190, 201, 207, 255]));
        y += 30;
    }

    draw_metrics(&mut image, meta, accent);
    draw_text(
        &mut image,
        &compact_url(&meta.canonical_url),
        96,
        560,
        2,
        Rgba([132, 149, 158, 255]),
    );

    let mut cursor = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image).write_to(&mut cursor, ImageFormat::Png)?;
    Ok(cursor.into_inner())
}

fn draw_background(image: &mut RgbaImage, accent: Color) {
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let xf = x as f32 / WIDTH as f32;
            let yf = y as f32 / HEIGHT as f32;
            let glow = ((1.0 - xf) * 0.22 + (1.0 - yf) * 0.16).clamp(0.0, 1.0);
            let r = (14.0 + accent[0] as f32 * glow * 0.16) as u8;
            let g = (18.0 + accent[1] as f32 * glow * 0.14) as u8;
            let b = (20.0 + accent[2] as f32 * glow * 0.14) as u8;
            image.put_pixel(x, y, Rgba([r, g, b, 255]));
        }
    }

    fill_rect(image, 54, 48, 1092, 534, Rgba([18, 22, 24, 214]));
    draw_rect(image, 54, 48, 1092, 534, Rgba([58, 72, 78, 255]));
}

fn draw_pill(image: &mut RgbaImage, x: i32, y: i32, label: &str, accent: Color) {
    let text = label.to_ascii_uppercase();
    let width = text_width(&text, 2) + 36;
    fill_rect(image, x, y, width, 34, Rgba([24, 34, 39, 255]));
    draw_rect(image, x, y, width, 34, accent);
    draw_text(image, &text, x + 18, y + 9, 2, accent);
}

fn draw_metrics(image: &mut RgbaImage, meta: &EmbedMetadata, accent: Color) {
    let metrics = meta.metrics.iter().take(4).collect::<Vec<_>>();
    if metrics.is_empty() {
        return;
    }

    let card_width = 240;
    let gap = 18;
    let total_width = (metrics.len() as i32 * card_width) + ((metrics.len() as i32 - 1) * gap);
    let mut x = 96 + ((920 - total_width) / 2).max(0);

    for metric in metrics {
        fill_rect(image, x, 438, card_width, 82, Rgba([20, 25, 28, 235]));
        draw_rect(image, x, 438, card_width, 82, Rgba([48, 60, 66, 255]));
        fill_rect(image, x, 438, card_width, 4, accent);

        draw_text(
            image,
            &metric.label.to_ascii_uppercase(),
            x + 18,
            456,
            1,
            Rgba([142, 158, 166, 255]),
        );

        let value = truncate(&metric.value, 18);
        draw_text(image, &value, x + 18, 480, 2, Rgba([237, 245, 249, 255]));

        x += card_width + gap;
    }
}

fn draw_text(image: &mut RgbaImage, text: &str, x: i32, y: i32, scale: i32, color: Color) {
    let mut cursor_x = x;

    for ch in text.chars() {
        if ch == '\n' {
            cursor_x = x;
            continue;
        }

        draw_char(image, ch, cursor_x, y, scale, color);
        cursor_x += 8 * scale + scale;
    }
}

fn draw_char(image: &mut RgbaImage, ch: char, x: i32, y: i32, scale: i32, color: Color) {
    let glyph = BASIC_FONTS
        .get(ch)
        .or_else(|| BASIC_FONTS.get('?'))
        .unwrap_or([0; 8]);

    for (row, bits) in glyph.iter().enumerate() {
        for col in 0..8 {
            if bits & (1 << col) != 0 {
                fill_rect(
                    image,
                    x + col * scale,
                    y + row as i32 * scale,
                    scale,
                    scale,
                    color,
                );
            }
        }
    }
}

fn fill_rect(image: &mut RgbaImage, x: i32, y: i32, width: i32, height: i32, color: Color) {
    let min_x = x.max(0) as u32;
    let min_y = y.max(0) as u32;
    let max_x = (x + width).clamp(0, WIDTH as i32) as u32;
    let max_y = (y + height).clamp(0, HEIGHT as i32) as u32;

    for py in min_y..max_y {
        for px in min_x..max_x {
            image.put_pixel(px, py, color);
        }
    }
}

fn draw_rect(image: &mut RgbaImage, x: i32, y: i32, width: i32, height: i32, color: Color) {
    fill_rect(image, x, y, width, 2, color);
    fill_rect(image, x, y + height - 2, width, 2, color);
    fill_rect(image, x, y, 2, height, color);
    fill_rect(image, x + width - 2, y, 2, height, color);
}

fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        let current_len = current.chars().count();
        let word_len = word.chars().count();
        let next_len = if current.is_empty() {
            word_len
        } else {
            current_len + 1 + word_len
        };

        if next_len > max_chars && !current.is_empty() {
            lines.push(current);
            current = String::new();
        }

        if !current.is_empty() {
            current.push(' ');
        }

        if word_len > max_chars {
            current.push_str(&truncate(word, max_chars));
        } else {
            current.push_str(word);
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

fn text_width(text: &str, scale: i32) -> i32 {
    text.chars().count() as i32 * (8 * scale + scale)
}

fn truncate(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }

    let mut output: String = text.chars().take(max_chars.saturating_sub(1)).collect();
    output.push('…');
    output
}

fn compact_url(url: &str) -> String {
    url.trim_start_matches("https://")
        .trim_start_matches("http://")
        .to_string()
}

fn accent_for_kind(kind: &str) -> Color {
    match kind.to_ascii_lowercase().as_str() {
        "club" | "clubs" => Rgba([255, 183, 77, 255]),
        "profile" | "veterans" | "career menu" | "achievements" | "titles" => {
            Rgba([255, 64, 129, 255])
        }
        "timeline" => Rgba([129, 199, 132, 255]),
        "tierlist" => Rgba([255, 214, 102, 255]),
        "database" | "lineage planner" => Rgba([74, 168, 255, 255]),
        _ => Rgba([74, 168, 255, 255]),
    }
}

#[cfg(test)]
mod tests {
    use crate::embed::{EmbedMetadata, EmbedMetric, ResourceCatalog};

    use super::*;

    #[test]
    fn renders_png_card() {
        let meta = EmbedMetadata {
            title: "Test Club | uma.moe".to_string(),
            description: "A generated preview card for a club link.".to_string(),
            canonical_url: "https://uma.moe/circles/772781438".to_string(),
            image_url: "https://uma.moe/__embeds/images/circle/772781438.png".to_string(),
            image_alt: "Test image".to_string(),
            kind_label: "Club".to_string(),
            metrics: vec![EmbedMetric {
                label: "Rank".to_string(),
                value: "#42".to_string(),
            }],
            database: None,
            tierlist: None,
            resources: ResourceCatalog::default(),
        };

        let bytes = render_png(&meta).expect("card renders");
        assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
    }
}

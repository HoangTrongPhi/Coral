//! Alpha bleed: give fully transparent pixels the colour of their nearest opaque neighbours (rings
//! grow outward, breadth first), keeping alpha 0. Bilinear filtering and mipmaps then no longer pull
//! black or white fringes into sprite edges.

use std::collections::VecDeque;

use image::RgbaImage;

/// Bleed in place; `max_rings` limits how far colour travels (0 = until the image is full).
pub fn alpha_bleed(img: &mut RgbaImage, max_rings: u32) {
    let (w, h) = img.dimensions();
    let (w, h) = (w as i64, h as i64);
    let idx = |x: i64, y: i64| (y * w + x) as usize;
    // 0 = has colour, 1 = waiting, 2 = queued.
    let mut state: Vec<u8> = img.pixels().map(|p| if p[3] > 0 { 0 } else { 1 }).collect();
    let mut frontier: VecDeque<(i64, i64)> = VecDeque::new();
    const N8: [(i64, i64); 8] = [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)];
    for y in 0..h {
        for x in 0..w {
            if state[idx(x, y)] != 1 {
                continue;
            }
            if N8.iter().any(|(dx, dy)| {
                let (nx, ny) = (x + dx, y + dy);
                nx >= 0 && ny >= 0 && nx < w && ny < h && state[idx(nx, ny)] == 0
            }) {
                state[idx(x, y)] = 2;
                frontier.push_back((x, y));
            }
        }
    }
    let mut ring = 0;
    while !frontier.is_empty() && (max_rings == 0 || ring < max_rings) {
        ring += 1;
        let current: Vec<(i64, i64)> = frontier.drain(..).collect();
        // Average the coloured neighbours of the whole ring first, then mark it coloured.
        let colours: Vec<[u8; 3]> = current
            .iter()
            .map(|&(x, y)| {
                let (mut r, mut g, mut b, mut n) = (0u32, 0u32, 0u32, 0u32);
                for (dx, dy) in N8 {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx >= 0 && ny >= 0 && nx < w && ny < h && state[idx(nx, ny)] == 0 {
                        let p = img.get_pixel(nx as u32, ny as u32);
                        r += p[0] as u32;
                        g += p[1] as u32;
                        b += p[2] as u32;
                        n += 1;
                    }
                }
                let n = n.max(1);
                [(r / n) as u8, (g / n) as u8, (b / n) as u8]
            })
            .collect();
        for (&(x, y), c) in current.iter().zip(&colours) {
            let p = img.get_pixel_mut(x as u32, y as u32);
            p[0] = c[0];
            p[1] = c[1];
            p[2] = c[2];
            state[idx(x, y)] = 0;
        }
        for &(x, y) in &current {
            for (dx, dy) in N8 {
                let (nx, ny) = (x + dx, y + dy);
                if nx >= 0 && ny >= 0 && nx < w && ny < h && state[idx(nx, ny)] == 1 {
                    state[idx(nx, ny)] = 2;
                    frontier.push_back((nx, ny));
                }
            }
        }
    }
}

/// Multiply RGB by alpha (engines that expect premultiplied textures).
pub fn premultiply(img: &mut RgbaImage) {
    for p in img.pixels_mut() {
        let a = p[3] as u32;
        for c in 0..3 {
            p[c] = ((p[c] as u32 * a + 127) / 255) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    #[test]
    fn transparent_neighbours_take_the_colour() {
        let mut img = RgbaImage::new(5, 1);
        img.put_pixel(0, 0, Rgba([255, 0, 0, 255]));
        alpha_bleed(&mut img, 0);
        for x in 1..5 {
            assert_eq!(*img.get_pixel(x, 0), Rgba([255, 0, 0, 0]), "x={x}");
        }
        let mut limited = RgbaImage::new(5, 1);
        limited.put_pixel(0, 0, Rgba([0, 255, 0, 255]));
        alpha_bleed(&mut limited, 2);
        assert_eq!(limited.get_pixel(2, 0)[1], 255);
        assert_eq!(limited.get_pixel(3, 0)[1], 0, "stopped after 2 rings");
        let mut p = RgbaImage::from_pixel(1, 1, Rgba([200, 100, 50, 128]));
        premultiply(&mut p);
        assert_eq!(*p.get_pixel(0, 0), Rgba([100, 50, 25, 128]));
    }
}

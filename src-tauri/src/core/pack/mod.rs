//! Atlas layout: the smallest page that holds everything (power of two / square on request),
//! padding between sprites, extrusion around them, more pages when one is not enough.
//!
//! Each sprite occupies a cell of `w + 2·extrude + padding` × `h + 2·extrude + padding` in a bin of
//! `page + padding`, so neighbours are always `padding + 2·extrude` apart while sprites may touch the page
//! edge. The sprite's own pixels start `extrude` px inside its cell.

pub mod maxrects;

use maxrects::MaxRects;

use crate::core::project::{Heuristic, PackSettings};

#[derive(Debug, Clone)]
pub struct Item {
    /// Caller's index.
    pub id: usize,
    pub w: u32,
    pub h: u32,
    /// Tie-break for the deterministic order.
    pub key: String,
}

/// Where an item went: page and top-left of its pixels (not of its cell).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placed {
    pub id: usize,
    pub page: usize,
    pub x: u32,
    pub y: u32,
    /// Stored turned 90° clockwise: occupies `h`×`w` in the page.
    pub rotated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageSize {
    pub w: u32,
    pub h: u32,
}

#[derive(Debug, Clone)]
pub struct Layout {
    pub pages: Vec<PageSize>,
    pub placed: Vec<Placed>,
    /// Items larger than `maxSize`, or all that did not fit without `multiPage`.
    pub rejected: Vec<usize>,
}

struct Cfg {
    pad: u32,
    ext: u32,
    heuristic: Heuristic,
    rotate: bool,
}

impl Cfg {
    fn cell(&self, it: &Item) -> (u32, u32) {
        (it.w + 2 * self.ext + self.pad, it.h + 2 * self.ext + self.pad)
    }
}

/// Pack all `items` into one `w`×`h` page, or None.
fn try_page(items: &[&Item], w: u32, h: u32, c: &Cfg) -> Option<Vec<(usize, u32, u32, bool)>> {
    let mut bin = MaxRects::new(w + c.pad, h + c.pad);
    let mut out = Vec::with_capacity(items.len());
    for it in items {
        let (cw, ch) = c.cell(it);
        let p = bin.insert(cw, ch, c.heuristic, c.rotate)?;
        out.push((it.id, p.rect.x + c.ext, p.rect.y + c.ext, p.rotated));
    }
    Some(out)
}

/// Used extent of a placement (pixels + extrusion, no trailing padding).
fn extent(items: &[&Item], placed: &[(usize, u32, u32, bool)], c: &Cfg) -> (u32, u32) {
    let mut w = 0;
    let mut h = 0;
    for (it, &(_, x, y, rot)) in items.iter().zip(placed) {
        let (iw, ih) = if rot { (it.h, it.w) } else { (it.w, it.h) };
        w = w.max(x + iw + c.ext);
        h = h.max(y + ih + c.ext);
    }
    (w, h)
}

/// The smallest page for `items` within `max`, honouring power-of-two / square.
fn smallest_page(items: &[&Item], s: &PackSettings, c: &Cfg) -> Option<(PageSize, Vec<(usize, u32, u32, bool)>)> {
    let max = s.max_size.max(1);
    let area: u64 = items.iter().map(|it| {
        let (w, h) = c.cell(it);
        w as u64 * h as u64
    }).sum();
    let min_side = items.iter().map(|it| it.w.min(it.h) + 2 * c.ext).max().unwrap_or(1);
    let (min_w, min_h) = if c.rotate {
        (min_side, min_side)
    } else {
        (items.iter().map(|it| it.w + 2 * c.ext).max().unwrap_or(1), items.iter().map(|it| it.h + 2 * c.ext).max().unwrap_or(1))
    };
    if min_w > max || min_h > max {
        return None;
    }

    if s.power_of_two {
        let sizes: Vec<u32> = std::iter::successors(Some(1u32), |v| (*v < max).then(|| v * 2)).filter(|&v| v <= max).collect();
        let mut combos: Vec<(u32, u32)> = sizes.iter().flat_map(|&w| sizes.iter().map(move |&h| (w, h))).filter(|&(w, h)| (!s.square || w == h) && w >= min_w && h >= min_h && (w as u64 + c.pad as u64) * (h as u64 + c.pad as u64) >= area).collect();
        combos.sort_by_key(|&(w, h)| (w as u64 * h as u64, w.max(h), std::cmp::Reverse(w)));
        for (w, h) in combos {
            if let Some(p) = try_page(items, w, h, c) {
                return Some((PageSize { w, h }, p));
            }
        }
        return None;
    }

    // Free sizes: try a spread of widths, each in a tall page, and keep the smallest cropped result.
    let lower = ((area as f64).sqrt() as u32).max(min_w).min(max);
    let mut widths: Vec<u32> = (0..=16).map(|i| lower + (max - lower) * i / 16).collect();
    widths.extend([min_w, max]);
    widths.sort();
    widths.dedup();
    let mut best: Option<(u64, PageSize, Vec<(usize, u32, u32, bool)>)> = None;
    for w in widths {
        let h_limit = if s.square { w } else { max };
        if let Some(p) = try_page(items, w, h_limit, c) {
            let (ew, eh) = extent(items, &p, c);
            let (pw, ph) = if s.square { (ew.max(eh), ew.max(eh)) } else { (ew, eh) };
            let a = pw as u64 * ph as u64;
            if best.as_ref().is_none_or(|b| a < b.0) {
                best = Some((a, PageSize { w: pw, h: ph }, p));
            }
        }
    }
    best.map(|(_, s, p)| (s, p))
}

/// Lay out `items` (trimmed sizes, already deduplicated).
pub fn layout(items: &[Item], s: &PackSettings) -> Layout {
    let c = Cfg { pad: s.padding, ext: s.extrude, heuristic: s.heuristic, rotate: s.allow_rotation };
    // Largest first, then by name: the same input always gives the same atlas.
    let mut order: Vec<&Item> = items.iter().collect();
    order.sort_by(|a, b| (b.w as u64 * b.h as u64).cmp(&(a.w as u64 * a.h as u64)).then_with(|| crate::core::natural::cmp(&a.key, &b.key)).then(a.id.cmp(&b.id)));

    let max = s.max_size.max(1);
    // Pages are at most max × max, so rotation never helps a sprite that is too large.
    let fits_alone = |it: &Item| it.w + 2 * c.ext <= max && it.h + 2 * c.ext <= max;
    let mut rejected: Vec<usize> = order.iter().filter(|it| !fits_alone(it)).map(|it| it.id).collect();
    order.retain(|it| fits_alone(it));

    let mut out = Layout { pages: Vec::new(), placed: Vec::new(), rejected: Vec::new() };
    let push_page = |size: PageSize, placed: Vec<(usize, u32, u32, bool)>, out: &mut Layout| {
        let page = out.pages.len();
        out.pages.push(size);
        out.placed.extend(placed.into_iter().map(|(id, x, y, rotated)| Placed { id, page, x, y, rotated }));
    };

    if order.is_empty() {
        out.rejected = rejected;
        return out;
    }
    if let Some((size, placed)) = smallest_page(&order, s, &c) {
        push_page(size, placed, &mut out);
        out.rejected = rejected;
        return out;
    }
    if !s.multi_page {
        rejected.extend(order.iter().map(|it| it.id));
        out.rejected = rejected;
        return out;
    }
    // Fill full-size pages greedily, then shrink each page to what it holds.
    let mut rest = order;
    while !rest.is_empty() {
        let mut bin = MaxRects::new(max + c.pad, max + c.pad);
        let (mut here, mut later) = (Vec::new(), Vec::new());
        for it in rest {
            let (cw, ch) = c.cell(it);
            if bin.insert(cw, ch, c.heuristic, c.rotate).is_some() {
                here.push(it);
            } else {
                later.push(it);
            }
        }
        if here.is_empty() {
            rejected.extend(later.iter().map(|it| it.id));
            break;
        }
        match smallest_page(&here, s, &c) {
            Some((size, placed)) => push_page(size, placed, &mut out),
            None => {
                let placed = try_page(&here, max, max, &c).unwrap_or_default();
                push_page(PageSize { w: max, h: max }, placed, &mut out);
            }
        }
        rest = later;
    }
    out.rejected = rejected;
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Small deterministic generator (xorshift), so the test needs no rand crate.
    fn rng(seed: &mut u64) -> u32 {
        *seed ^= *seed << 13;
        *seed ^= *seed >> 7;
        *seed ^= *seed << 17;
        (*seed >> 32) as u32
    }

    fn items(n: usize, seed: u64, max: u32) -> Vec<Item> {
        let mut s = seed;
        (0..n).map(|i| Item { id: i, w: 1 + rng(&mut s) % max, h: 1 + rng(&mut s) % max, key: format!("s{i}") }).collect()
    }

    fn check(items: &[Item], s: &PackSettings, l: &Layout) {
        let (pad, ext) = (s.padding, s.extrude);
        assert!(l.rejected.is_empty(), "everything placed: {:?}", l.rejected);
        assert_eq!(l.placed.len(), items.len());
        // Cells: pixels + extrusion on all sides + padding to the right / bottom.
        let cells: Vec<(usize, maxrects::Rect)> = l
            .placed
            .iter()
            .map(|p| {
                let it = &items[p.id];
                let (w, h) = if p.rotated { (it.h, it.w) } else { (it.w, it.h) };
                let page = &l.pages[p.page];
                assert!(p.x >= ext && p.y >= ext && p.x + w + ext <= page.w && p.y + h + ext <= page.h, "inside the page");
                (p.page, maxrects::Rect::new(p.x - ext, p.y - ext, w + 2 * ext + pad, h + 2 * ext + pad))
            })
            .collect();
        for (i, (pa, a)) in cells.iter().enumerate() {
            for (pb, b) in &cells[i + 1..] {
                assert!(pa != pb || !a.intersects(b), "overlap {a:?} {b:?}");
            }
        }
    }

    #[test]
    fn five_hundred_random_rects_never_overlap() {
        for (heuristic, rot, pot) in [(Heuristic::BestShortSideFit, false, true), (Heuristic::BestAreaFit, true, true), (Heuristic::ContactPoint, false, false), (Heuristic::BottomLeft, true, false), (Heuristic::BestLongSideFit, false, true)] {
            let it = items(500, 0x9E37_79B9_7F4A_7C15, 48);
            let s = PackSettings { padding: 2, extrude: 1, heuristic, allow_rotation: rot, power_of_two: pot, max_size: 1024, ..Default::default() };
            let l = layout(&it, &s);
            check(&it, &s, &l);
            if pot {
                assert!(l.pages.iter().all(|p| p.w.is_power_of_two() && p.h.is_power_of_two()));
            }
        }
    }

    #[test]
    fn deterministic_multi_page_and_limits() {
        let it = items(200, 42, 60);
        let s = PackSettings { max_size: 256, ..Default::default() };
        let a = layout(&it, &s);
        let b = layout(&it, &s);
        assert_eq!(a.placed, b.placed);
        assert!(a.pages.len() > 1, "needs several 256 pages");
        check(&it, &s, &a);

        let single = layout(&it, &PackSettings { max_size: 256, multi_page: false, ..Default::default() });
        assert_eq!(single.rejected.len(), 200, "no partial single page");
        let big = vec![Item { id: 0, w: 300, h: 10, key: "big".into() }];
        assert_eq!(layout(&big, &s).rejected, [0]);

        let sq = layout(&items(30, 7, 20), &PackSettings { square: true, ..Default::default() });
        assert!(sq.pages.iter().all(|p| p.w == p.h));
        let free = layout(&[Item { id: 0, w: 30, h: 44, key: "a".into() }], &PackSettings { power_of_two: false, padding: 0, extrude: 0, ..Default::default() });
        assert_eq!(free.pages[0], PageSize { w: 30, h: 44 });
    }

    #[test]
    #[ignore = "benchmark: cargo test --release -- --ignored bench"]
    fn bench_1000_sprites_64px() {
        let it: Vec<Item> = (0..1000).map(|i| Item { id: i, w: 64, h: 64, key: format!("s{i}") }).collect();
        let t = std::time::Instant::now();
        let l = layout(&it, &PackSettings { max_size: 4096, ..Default::default() });
        let ms = t.elapsed().as_millis();
        println!("1000 × 64px: {} page(s) {:?} in {ms} ms", l.pages.len(), l.pages[0]);
        assert!(ms < 300, "{ms} ms");
    }
}

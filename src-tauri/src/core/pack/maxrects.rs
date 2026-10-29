//! MaxRects bin packing (Jukka Jylänki, "A Thousand Ways to Pack the Bin", 2010): free rectangles,
//! split on placement, prune contained ones. Five placement heuristics, optional 90° rotation.
//! Deterministic: same input order, same result.

use crate::core::project::Heuristic;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl Rect {
    pub fn new(x: u32, y: u32, w: u32, h: u32) -> Self {
        Rect { x, y, w, h }
    }
    pub fn right(&self) -> u32 {
        self.x + self.w
    }
    pub fn bottom(&self) -> u32 {
        self.y + self.h
    }
    pub fn contains(&self, o: &Rect) -> bool {
        o.x >= self.x && o.y >= self.y && o.right() <= self.right() && o.bottom() <= self.bottom()
    }
    pub fn intersects(&self, o: &Rect) -> bool {
        self.x < o.right() && o.x < self.right() && self.y < o.bottom() && o.y < self.bottom()
    }
}

pub struct MaxRects {
    pub w: u32,
    pub h: u32,
    free: Vec<Rect>,
    used: Vec<Rect>,
}

/// A placement: the rect in the bin (rotated size when `rotated`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placement {
    pub rect: Rect,
    pub rotated: bool,
}

impl MaxRects {
    pub fn new(w: u32, h: u32) -> Self {
        MaxRects { w, h, free: vec![Rect::new(0, 0, w, h)], used: Vec::new() }
    }

    pub fn used_area(&self) -> u64 {
        self.used.iter().map(|r| r.w as u64 * r.h as u64).sum()
    }

    /// Place a `w`×`h` rect, or None when it fits nowhere.
    pub fn insert(&mut self, w: u32, h: u32, heuristic: Heuristic, allow_rotation: bool) -> Option<Placement> {
        let best = self.find(w, h, heuristic, allow_rotation)?;
        self.place(best.rect);
        Some(best)
    }

    fn score(&self, r: &Rect, free: &Rect, heuristic: Heuristic) -> (i64, i64) {
        let (lw, lh) = (free.w as i64 - r.w as i64, free.h as i64 - r.h as i64);
        let (short, long) = (lw.min(lh), lw.max(lh));
        match heuristic {
            Heuristic::BestShortSideFit => (short, long),
            Heuristic::BestLongSideFit => (long, short),
            Heuristic::BestAreaFit => (free.w as i64 * free.h as i64 - r.w as i64 * r.h as i64, short),
            Heuristic::BottomLeft => (r.bottom() as i64, r.x as i64),
            // Higher contact is better: negate so smaller wins everywhere.
            Heuristic::ContactPoint => (-(self.contact(r) as i64), r.y as i64),
        }
    }

    fn contact(&self, r: &Rect) -> u32 {
        let overlap = |a0: u32, a1: u32, b0: u32, b1: u32| a1.min(b1).saturating_sub(a0.max(b0));
        let mut s = 0;
        if r.x == 0 || r.right() == self.w {
            s += r.h;
        }
        if r.y == 0 || r.bottom() == self.h {
            s += r.w;
        }
        for u in &self.used {
            if u.x == r.right() || u.right() == r.x {
                s += overlap(u.y, u.bottom(), r.y, r.bottom());
            }
            if u.y == r.bottom() || u.bottom() == r.y {
                s += overlap(u.x, u.right(), r.x, r.right());
            }
        }
        s
    }

    fn find(&self, w: u32, h: u32, heuristic: Heuristic, allow_rotation: bool) -> Option<Placement> {
        let mut best: Option<((i64, i64), Placement)> = None;
        for f in &self.free {
            for (rw, rh, rotated) in [(w, h, false), (h, w, true)] {
                if rotated && (!allow_rotation || w == h) {
                    continue;
                }
                if rw <= f.w && rh <= f.h {
                    let rect = Rect::new(f.x, f.y, rw, rh);
                    let s = self.score(&rect, f, heuristic);
                    if best.as_ref().is_none_or(|(b, _)| s < *b) {
                        best = Some((s, Placement { rect, rotated }));
                    }
                }
            }
        }
        best.map(|(_, p)| p)
    }

    fn place(&mut self, r: Rect) {
        let mut next = Vec::with_capacity(self.free.len() + 4);
        for f in self.free.drain(..) {
            if !f.intersects(&r) {
                next.push(f);
                continue;
            }
            // Split the free rect into up to four maximal pieces around `r`.
            if r.x > f.x {
                next.push(Rect::new(f.x, f.y, r.x - f.x, f.h));
            }
            if r.right() < f.right() {
                next.push(Rect::new(r.right(), f.y, f.right() - r.right(), f.h));
            }
            if r.y > f.y {
                next.push(Rect::new(f.x, f.y, f.w, r.y - f.y));
            }
            if r.bottom() < f.bottom() {
                next.push(Rect::new(f.x, r.bottom(), f.w, f.bottom() - r.bottom()));
            }
        }
        // Prune rects contained in others (keep the first of identical ones).
        let mut keep = vec![true; next.len()];
        for i in 0..next.len() {
            if !keep[i] {
                continue;
            }
            for j in 0..next.len() {
                if i != j && keep[j] && next[j].contains(&next[i]) && (next[i] != next[j] || j < i) {
                    keep[i] = false;
                    break;
                }
            }
        }
        self.free = next.into_iter().zip(keep).filter_map(|(r, k)| k.then_some(r)).collect();
        self.used.push(r);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fills_exactly_and_refuses_overflow() {
        for h in [Heuristic::BestShortSideFit, Heuristic::BestLongSideFit, Heuristic::BestAreaFit, Heuristic::BottomLeft, Heuristic::ContactPoint] {
            let mut b = MaxRects::new(64, 64);
            let mut placed = Vec::new();
            for _ in 0..16 {
                placed.push(b.insert(16, 16, h, false).unwrap_or_else(|| panic!("{h:?}")).rect);
            }
            assert!(b.insert(1, 1, h, false).is_none(), "{h:?}: full");
            for (i, a) in placed.iter().enumerate() {
                assert!(placed[i + 1..].iter().all(|o| !a.intersects(o)));
            }
        }
        let mut b = MaxRects::new(10, 40);
        assert!(b.insert(40, 10, Heuristic::BestShortSideFit, false).is_none());
        let p = b.insert(40, 10, Heuristic::BestShortSideFit, true).unwrap();
        assert!(p.rotated && p.rect.w == 10 && p.rect.h == 40);
    }
}

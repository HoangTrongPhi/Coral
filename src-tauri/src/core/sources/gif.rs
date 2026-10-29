//! Animated GIF: every frame a sprite `<file>_<i>`, delays → per-frame durations, one animation `<file>`.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;

use image::codecs::gif::GifDecoder;
use image::AnimationDecoder;

use super::{Loaded, SourceRef, SourceSprite};
use crate::core::project::LoopMode;
use crate::core::{anim, Issue};

pub fn load(path: &Path) -> Loaded {
    load_named(path, &super::file_stem(path))
}

pub fn load_named(path: &Path, base: &str) -> Loaded {
    let frames = File::open(path).map_err(|e| e.to_string()).and_then(|f| {
        let dec = GifDecoder::new(BufReader::new(f)).map_err(|e| e.to_string())?;
        dec.into_frames().collect_frames().map_err(|e| e.to_string())
    });
    let frames = match frames {
        Ok(f) => f,
        Err(e) => return Loaded { issues: vec![Issue::error("unreadable-gif", format!("{}: {e}", path.display())).at(path)], ..Default::default() },
    };
    let mut out = Loaded::default();
    let mut durations = Vec::new();
    for (i, fr) in frames.into_iter().enumerate() {
        let (num, den) = fr.delay().numer_denom_ms();
        let ms = if den == 0 { 100 } else { (num / den).max(10) };
        durations.push(ms);
        out.sprites.push(SourceSprite {
            name: format!("{base}_{i}"),
            image: Arc::new(fr.into_buffer()),
            origin: SourceRef { path: path.to_string_lossy().into_owned(), frame: Some(i as u32), layer: None },
            duration_ms: Some(ms),
            pivot: None,
            nine_slice: None,
        });
    }
    if out.sprites.len() > 1 {
        let names = out.sprites.iter().map(|s| s.name.clone()).collect();
        out.animations.push(anim::from_durations(base.to_string(), names, durations, LoopMode::Loop));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::codecs::gif::{GifEncoder, Repeat};
    use image::{Delay, Frame};

    #[test]
    fn frames_and_delays() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("blink.gif");
        {
            let mut enc = GifEncoder::new(File::create(&p).unwrap());
            enc.set_repeat(Repeat::Infinite).unwrap();
            for (i, ms) in [100u32, 100, 300].iter().enumerate() {
                let img = crate::core::sources::test_support::shape(6, 6, i as u32, 0, 2, 2, [255, 0, 0, 255]);
                enc.encode_frame(Frame::from_parts(img, 0, 0, Delay::from_numer_denom_ms(*ms, 1))).unwrap();
            }
        }
        let l = load(&p);
        assert!(l.issues.is_empty(), "{:?}", l.issues);
        assert_eq!(l.sprites.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), ["blink_0", "blink_1", "blink_2"]);
        assert_eq!(l.animations[0].fps, 10.0);
        assert_eq!(l.animations[0].durations_ms, Some(vec![100, 100, 300]));
    }
}

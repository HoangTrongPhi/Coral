//! Animations: from Aseprite tags and from numbered file names, then the project's own on top.

use std::collections::BTreeMap;

use super::natural;
use super::project::{Animation, LoopMode, Project};

/// Group `name_0..name_n` / `name0001…` into animations (2+ frames), frames in natural order.
pub fn from_names(names: &[String], fps: f32) -> Vec<Animation> {
    let mut groups: BTreeMap<String, Vec<(u64, String)>> = BTreeMap::new();
    for n in names {
        if let (base, Some(i)) = natural::split_number(n) {
            groups.entry(base.to_string()).or_default().push((i, n.clone()));
        }
    }
    let mut out: Vec<Animation> = groups
        .into_iter()
        .filter(|(_, f)| f.len() >= 2)
        .map(|(name, mut frames)| {
            frames.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| natural::cmp(&a.1, &b.1)));
            Animation { name, frames: frames.into_iter().map(|f| f.1).collect(), fps, loop_mode: LoopMode::Loop, durations_ms: None }
        })
        .collect();
    out.sort_by(|a, b| natural::cmp(&a.name, &b.name));
    out
}

/// The most common frame time as fps, the rest as per-frame overrides (when they differ).
pub fn from_durations(name: String, frames: Vec<String>, durations: Vec<u32>, loop_mode: LoopMode) -> Animation {
    let mut counts: BTreeMap<u32, usize> = BTreeMap::new();
    for &d in &durations {
        *counts.entry(d.max(1)).or_default() += 1;
    }
    let common = counts.iter().max_by_key(|(d, c)| (**c, std::cmp::Reverse(**d))).map(|(d, _)| *d).unwrap_or(100);
    let fps = (1000.0 / common as f32 * 100.0).round() / 100.0;
    let uniform = durations.iter().all(|&d| d.max(1) == common);
    Animation { name, frames, fps, loop_mode, durations_ms: (!uniform).then_some(durations) }
}

/// Automatic animations, replaced by the project's ones with the same name; the project's extra ones
/// are kept. Frames that do not exist are dropped (the caller reports them).
pub fn merge(auto: Vec<Animation>, project: &Project) -> Vec<Animation> {
    let mut by_name: BTreeMap<String, Animation> = auto.into_iter().map(|a| (a.name.clone(), a)).collect();
    for a in &project.animations {
        by_name.insert(a.name.clone(), a.clone());
    }
    let mut v: Vec<Animation> = by_name.into_values().collect();
    v.sort_by(|a, b| natural::cmp(&a.name, &b.name));
    v
}

/// Frames in play order for engines without ping-pong: 0 1 2 3 → 0 1 2 3 2 1.
pub fn expand_pingpong<T: Clone>(frames: &[T]) -> Vec<T> {
    let mut v = frames.to_vec();
    if frames.len() > 2 {
        v.extend(frames[1..frames.len() - 1].iter().rev().cloned());
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_numbered_names() {
        let names: Vec<String> = ["run_10", "run_2", "run_1", "idle0001", "idle0002", "logo", "fx/spark_0", "fx/spark_1", "single_1"].iter().map(|s| s.to_string()).collect();
        let a = from_names(&names, 12.0);
        let names: Vec<&str> = a.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, ["fx/spark", "idle", "run"]);
        assert_eq!(a[2].frames, ["run_1", "run_2", "run_10"]);
        assert_eq!(expand_pingpong(&[0, 1, 2, 3]), [0, 1, 2, 3, 2, 1]);
    }

    #[test]
    fn durations_to_fps() {
        let a = from_durations("x".into(), vec!["a".into(), "b".into(), "c".into()], vec![100, 100, 300], LoopMode::Loop);
        assert_eq!(a.fps, 10.0);
        assert_eq!(a.durations_ms, Some(vec![100, 100, 300]));
        assert!(from_durations("y".into(), vec!["a".into()], vec![125], LoopMode::Once).durations_ms.is_none());
    }
}

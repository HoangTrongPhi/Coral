//! Natural sort: `run_2` before `run_10`, case-insensitive, digits compared as numbers.

use std::cmp::Ordering;
use std::path::Path;

fn chunks(s: &str) -> Vec<(bool, &str)> {
    let mut out = Vec::new();
    let mut start = 0;
    let bytes = s.as_bytes();
    for i in 1..=bytes.len() {
        let boundary = i == bytes.len() || bytes[i].is_ascii_digit() != bytes[i - 1].is_ascii_digit();
        if boundary {
            out.push((bytes[start].is_ascii_digit(), &s[start..i]));
            start = i;
        }
    }
    out
}

pub fn cmp(a: &str, b: &str) -> Ordering {
    if a.is_empty() || b.is_empty() {
        return a.cmp(b);
    }
    let (ca, cb) = (chunks(a), chunks(b));
    for ((da, sa), (db, sb)) in ca.iter().zip(&cb) {
        let o = match (da, db) {
            (true, true) => {
                let (ta, tb) = (sa.trim_start_matches('0'), sb.trim_start_matches('0'));
                ta.len().cmp(&tb.len()).then_with(|| ta.cmp(tb)).then_with(|| sa.len().cmp(&sb.len()))
            }
            _ => sa.to_lowercase().cmp(&sb.to_lowercase()),
        };
        if o != Ordering::Equal {
            return o;
        }
    }
    ca.len().cmp(&cb.len()).then_with(|| a.cmp(b))
}

pub fn cmp_path(a: &Path, b: &Path) -> Ordering {
    cmp(&a.to_string_lossy().replace('\\', "/"), &b.to_string_lossy().replace('\\', "/"))
}

/// `run_07` → (`run`, Some(7)); `walk0001` → (`walk`, Some(1)); `idle` → (`idle`, None).
/// The base loses one trailing `_`, `-`, `.` or space before the number.
pub fn split_number(name: &str) -> (&str, Option<u64>) {
    let digits = name.bytes().rev().take_while(u8::is_ascii_digit).count();
    if digits == 0 || digits == name.len() {
        return (name, None);
    }
    let (base, num) = name.split_at(name.len() - digits);
    let base = base.strip_suffix(['_', '-', '.', ' ']).unwrap_or(base);
    (base, num.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorts_numbers_as_numbers() {
        let mut v = vec!["run_10", "run_2", "Run_1", "idle", "run_02b"];
        v.sort_by(|a, b| cmp(a, b));
        assert_eq!(v, ["idle", "Run_1", "run_2", "run_02b", "run_10"]);
        assert_eq!(split_number("explosion_0064"), ("explosion", Some(64)));
        assert_eq!(split_number("walk0001"), ("walk", Some(1)));
        assert_eq!(split_number("idle"), ("idle", None));
        assert_eq!(split_number("0012"), ("0012", None));
    }
}

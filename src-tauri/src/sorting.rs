//! 自然排序（Natural Sort）
//!
//! 確保頁面以人類直覺排序：1, 2, ..., 9, 10, 11，而非字典序的 1, 10, 11, 2。
//! 數字段以「數值大小」比較，其餘字元以不分大小寫比較。

use std::cmp::Ordering;

/// 比較兩個字串的自然順序。
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let mut ai = a.chars().peekable();
    let mut bi = b.chars().peekable();

    loop {
        match (ai.peek().copied(), bi.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(ca), Some(cb)) => {
                if ca.is_ascii_digit() && cb.is_ascii_digit() {
                    let na = take_digits(&mut ai);
                    let nb = take_digits(&mut bi);
                    match compare_numeric(&na, &nb) {
                        Ordering::Equal => continue,
                        other => return other,
                    }
                } else {
                    let la = ca.to_ascii_lowercase();
                    let lb = cb.to_ascii_lowercase();
                    if la != lb {
                        return la.cmp(&lb);
                    }
                    ai.next();
                    bi.next();
                }
            }
        }
    }
}

/// 取出連續的數字字元。
fn take_digits<I: Iterator<Item = char>>(it: &mut std::iter::Peekable<I>) -> String {
    let mut s = String::new();
    while let Some(&c) = it.peek() {
        if c.is_ascii_digit() {
            s.push(c);
            it.next();
        } else {
            break;
        }
    }
    s
}

/// 比較兩串純數字字串的數值大小（忽略前導零）。
fn compare_numeric(a: &str, b: &str) -> Ordering {
    let ta = a.trim_start_matches('0');
    let tb = b.trim_start_matches('0');
    // 有效位數較多者較大
    match ta.len().cmp(&tb.len()) {
        Ordering::Equal => match ta.cmp(tb) {
            // 數值相等時，前導零較少者排前（穩定、可預期）
            Ordering::Equal => a.len().cmp(&b.len()),
            other => other,
        },
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sorted(mut v: Vec<&str>) -> Vec<&str> {
        v.sort_by(|a, b| natural_cmp(a, b));
        v
    }

    #[test]
    fn numeric_order() {
        assert_eq!(
            sorted(vec!["10.jpg", "1.jpg", "2.jpg", "11.jpg"]),
            vec!["1.jpg", "2.jpg", "10.jpg", "11.jpg"]
        );
    }

    #[test]
    fn leading_zeros() {
        assert_eq!(
            sorted(vec!["page008", "page010", "page009"]),
            vec!["page008", "page009", "page010"]
        );
    }

    #[test]
    fn mixed_text() {
        assert_eq!(
            sorted(vec!["ch2_p1", "ch10_p1", "ch1_p2"]),
            vec!["ch1_p2", "ch2_p1", "ch10_p1"]
        );
    }
}

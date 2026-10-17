//! Scientific annotations and imported numeric literals use true exponents.
use std::sync::OnceLock;
use regex::Regex;

/// Convert e-notation literals to LaTeX, preserving surrounding math/text mode.
pub fn scientific_text(text: &str) -> String {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    let re = PATTERN.get_or_init(|| Regex::new(r"(?P<m>[+-]?(?:\d+\.?\d*|\.\d+))[eE](?P<e>[+-]?\d+)").unwrap());
    let mut out = String::new();
    let mut end = 0;
    for c in re.captures_iter(text) {
        let whole = c.get(0).unwrap();
        out.push_str(&text[end..whole.start()]);
        let math = text[..whole.start()].chars().filter(|&c| c == '$').count()%2 == 1;
        let exponent = c["e"].parse::<i32>().unwrap();
        if !math { out.push('$'); }
        out.push_str(&format!("{}\\times10^{{{exponent}}}", &c["m"]));
        if !math { out.push('$'); }
        end = whole.end();
    }
    out.push_str(&text[end..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scientific_literals_in_both_modes() {
        assert_eq!(scientific_text("m = 3.54e-11 kg"), "m = $3.54\\times10^{-11}$ kg");
        assert_eq!(scientific_text("$m=1.5e-09$"), "$m=1.5\\times10^{-9}$");
        assert_eq!(scientific_text("95%: 1e-11 to 2E+03"), "95%: $1\\times10^{-11}$ to $2\\times10^{3}$");
    }
}

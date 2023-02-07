use regex::Regex;

/// Extracts regexp captures
///
/// Extract from `s` the captures defined in `reg_exp`
pub fn get_regexp_captures<'a>(s: &'a str, reg_exp: &str) -> Option<Vec<&'a str>> {
    Regex::new(reg_exp).map_or_else(
        |_| None,
        |re| {
            let mut res: Vec<&str> = Vec::new();
            re.captures_iter(s).for_each(|captures| {
                for i in 1..captures.len() {
                    res.push(captures.get(i).map_or_else(|| "", |m| m.as_str()));
                }
            });
            Some(res)
        },
    )
}

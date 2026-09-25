//! Repair thousands separators and decimal points in number-shaped OCR tokens
//! (xberg-io/xberg#1789).
//!
//! On a scanned financial table Tesseract often drops a thousands separator (`1172` for
//! `1,172`), reads a comma as a period (`7.812` for `7,812`), or splits a number at a gap
//! (`2 2,411` for `22,411`). The digits are right and the value is not. Three rules, each
//! applied only to a token that stands alone as a number, in this order:
//!
//! 1. A lone digit, a space, and `d,ddd` or `dd,ddd` join into one number.
//! 2. `d.ddd`, `dd.ddd` or `ddd.ddd`, exactly three digits after one period and no `%`
//!    after them, gets a comma instead of the period.
//! 3. A bare integer of 4 to 9 digits gets thousands separators, unless it follows `FY`,
//!    runs into `-` or `/`, or is followed by `%`.
//!
//! A four-digit number in prose is often a year or an identifier, which is why the caller
//! turns this on for tables and not by default.

/// Apply the three repairs to `text`, in order, and return the repaired text.
pub(crate) fn repair_numeric_tokens(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let joined = join_split_numbers(&chars);
    let with_commas = period_to_comma(&joined);
    group_thousands(&with_commas).into_iter().collect()
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn is_digit(c: char) -> bool {
    c.is_ascii_digit()
}

fn at(chars: &[char], i: usize) -> Option<char> {
    chars.get(i).copied()
}

fn prev_is(chars: &[char], i: usize, forbidden: &[char], word: bool) -> bool {
    match i.checked_sub(1).and_then(|p| at(chars, p)) {
        None => false,
        Some(c) => forbidden.contains(&c) || (word && is_word(c)),
    }
}

fn digit_run(chars: &[char], from: usize) -> usize {
    chars[from..].iter().take_while(|c| is_digit(**c)).count()
}

/// Rule 1: `(?<![\w.,])(\d) (\d{1,2},\d{3})(?![\d])` -> the two groups joined.
fn join_split_numbers(chars: &[char]) -> Vec<char> {
    let mut out = Vec::with_capacity(chars.len());
    let mut i = 0;
    while i < chars.len() {
        let matched = (|| {
            if !is_digit(at(chars, i)?) || prev_is(chars, i, &['.', ','], true) || at(chars, i + 1)? != ' ' {
                return None;
            }
            let lead = digit_run(chars, i + 2);
            if !(1..=2).contains(&lead) || at(chars, i + 2 + lead)? != ',' {
                return None;
            }
            let tail = i + 3 + lead;
            if digit_run(chars, tail) != 3 || at(chars, tail + 3).is_some_and(is_digit) {
                return None;
            }
            Some(tail + 3)
        })();
        match matched {
            Some(end) => {
                out.push(chars[i]);
                out.extend_from_slice(&chars[i + 2..end]);
                i = end;
            }
            None => {
                out.push(chars[i]);
                i += 1;
            }
        }
    }
    out
}

/// Rule 2: `(?<![\w.,])(\(?\$?\d{1,3})\.(\d{3})(?![\d%.,])` -> the period becomes a comma.
fn period_to_comma(chars: &[char]) -> Vec<char> {
    let mut out = Vec::with_capacity(chars.len());
    let mut i = 0;
    while i < chars.len() {
        let matched = (|| {
            if prev_is(chars, i, &['.', ','], true) {
                return None;
            }
            let mut j = i;
            if at(chars, j)? == '(' {
                j += 1;
            }
            if at(chars, j)? == '$' {
                j += 1;
            }
            let lead = digit_run(chars, j);
            if !(1..=3).contains(&lead) || at(chars, j + lead)? != '.' {
                return None;
            }
            let tail = j + lead + 1;
            if digit_run(chars, tail) != 3 {
                return None;
            }
            if at(chars, tail + 3).is_some_and(|c| is_digit(c) || matches!(c, '%' | '.' | ',')) {
                return None;
            }
            Some((j + lead, tail + 3))
        })();
        match matched {
            Some((period, end)) => {
                out.extend_from_slice(&chars[i..period]);
                out.push(',');
                out.extend_from_slice(&chars[period + 1..end]);
                i = end;
            }
            None => {
                out.push(chars[i]);
                i += 1;
            }
        }
    }
    out
}

/// Rule 3: `(?<![\w.,\-/])(?<!FY )(\(?)(\d{4,9})(\)?)(?![\w.,%\-/])` -> grouped digits.
fn group_thousands(chars: &[char]) -> Vec<char> {
    let mut out = Vec::with_capacity(chars.len() + 8);
    let mut i = 0;
    while i < chars.len() {
        let matched = (|| {
            if prev_is(chars, i, &['.', ',', '-', '/'], true) {
                return None;
            }
            if i >= 3 && chars[i - 3..i] == ['F', 'Y', ' '] {
                return None;
            }
            let mut j = i;
            if at(chars, j)? == '(' {
                j += 1;
            }
            let digits = digit_run(chars, j);
            if !(4..=9).contains(&digits) {
                return None;
            }
            let after_digits = j + digits;
            let blocked = |c: char| is_word(c) || matches!(c, '.' | ',' | '%' | '-' | '/');
            // Greedy optional `)`: with it when the lookahead then holds, else without it.
            let end = if at(chars, after_digits) == Some(')') && !at(chars, after_digits + 1).is_some_and(blocked) {
                after_digits + 1
            } else if !at(chars, after_digits).is_some_and(blocked) {
                after_digits
            } else {
                return None;
            };
            Some((j, after_digits, end))
        })();
        match matched {
            Some((start, after_digits, end)) => {
                out.extend_from_slice(&chars[i..start]);
                let digits = &chars[start..after_digits];
                for (k, digit) in digits.iter().enumerate() {
                    if k > 0 && (digits.len() - k).is_multiple_of(3) {
                        out.push(',');
                    }
                    out.push(*digit);
                }
                out.extend_from_slice(&chars[after_digits..end]);
                i = end;
            }
            None => {
                out.push(chars[i]);
                i += 1;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::repair_numeric_tokens;

    #[test]
    fn a_bare_integer_of_four_to_nine_digits_gets_separators() {
        assert_eq!(
            repair_numeric_tokens("APPLES 1172 48210 123456789"),
            "APPLES 1,172 48,210 123,456,789"
        );
        assert_eq!(repair_numeric_tokens("(2100)"), "(2,100)");
        assert_eq!(repair_numeric_tokens("x 123 1234567890 y"), "x 123 1234567890 y");
    }

    #[test]
    fn a_year_after_fy_a_range_a_ratio_and_a_percentage_are_left_alone() {
        assert_eq!(
            repair_numeric_tokens("FY 1999 and 1999-00 and 1/1999 and 1999%"),
            "FY 1999 and 1999-00 and 1/1999 and 1999%"
        );
        assert_eq!(
            repair_numeric_tokens("ref-1234 A1234 1234b 12.3456"),
            "ref-1234 A1234 1234b 12.3456"
        );
    }

    #[test]
    fn a_period_between_a_short_lead_and_three_digits_becomes_a_comma() {
        assert_eq!(
            repair_numeric_tokens("PEARS 7.812 $5.103 (2.100) 21.406"),
            "PEARS 7,812 $5,103 (2,100) 21,406"
        );
        assert_eq!(
            repair_numeric_tokens("3.1415 0.500% 1.234.5 1234.567"),
            "3.1415 0.500% 1.234.5 1234.567"
        );
    }

    #[test]
    fn a_split_leading_digit_rejoins_its_number() {
        assert_eq!(
            repair_numeric_tokens("SALT 2 2,411 and 1 12,000"),
            "SALT 22,411 and 112,000"
        );
        assert_eq!(
            repair_numeric_tokens("row 2 2,4111 and a2 2,411"),
            "row 2 2,4111 and a2 2,411"
        );
    }

    #[test]
    fn the_rules_compose_in_order_and_prose_without_numbers_is_untouched() {
        // Each token is handled by one rule, and the join runs before the separator rule, so
        // the joined "22,411" already carries its comma when that rule looks at it.
        assert_eq!(
            repair_numeric_tokens("2 2,411 and 7.812 and 1172"),
            "22,411 and 7,812 and 1,172"
        );
        // The join rule needs a comma-grouped tail; a period-separated tail is left for the
        // period rule alone, as in the reference order.
        assert_eq!(repair_numeric_tokens("2 2.411"), "2 2,411");
        let prose = "Opening stock, per the notes, was reviewed.";
        assert_eq!(repair_numeric_tokens(prose), prose);
        assert_eq!(repair_numeric_tokens(""), "");
    }
}

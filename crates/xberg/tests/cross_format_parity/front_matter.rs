pub(super) fn markdown_body_start(lines: &[&str]) -> usize {
    if lines.first() != Some(&"---") {
        return 0;
    }
    let Some(closing) = lines
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(i, line)| (*line == "---").then_some(i))
    else {
        return 0;
    };
    let yaml = lines[1..closing].join("\n");
    match yaml_serde::from_str::<yaml_serde::Value>(&yaml) {
        Ok(yaml_serde::Value::Mapping(_)) => closing + 1,
        _ => 0,
    }
}

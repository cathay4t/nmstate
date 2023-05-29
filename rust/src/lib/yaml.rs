// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use serde_yaml::Value;

use crate::{ErrorKind, NmstateError};

pub(crate) fn unknown_filed_to_string(
    iface_name: &str,
    iface_type: &str,
    field: &str,
) -> String {
    format!("unknown_field: interface.{iface_type}.{iface_name}.{field}")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub(crate) struct YamlPosition {
    pub(crate) line: usize,
    pub(crate) start: usize,
    pub(crate) end: usize,
}

impl YamlPosition {
    pub(crate) fn new(line: usize, start: usize, end: usize) -> Self {
        Self { line, start, end }
    }
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub(crate) enum SpannedValue {
    Null(YamlPosition),
    Bool(YamlPosition, bool),
    Number(YamlPosition, serde_yaml::Number),
    String(YamlPosition, String),
    Sequence(YamlPosition, Vec<SpannedValue>),
    Mapping(YamlPosition, HashMap<String, SpannedValue>),
    // Nmstate do not support tagged valued
}

pub(crate) fn deserialize_yaml_to_spanned_value(
    content: &str,
) -> Result<SpannedValue, NmstateError> {
    let lines: Vec<&str> = content.lines().map(str::trim_end).collect();
    _deserialize_yaml_to_spanned_value(&lines, 0, 0).map(
        |v, _| v)
}

fn _deserialize_yaml_to_spanned_value(
    lines: &[&str],
    line_offset: usize,
    indent: usize,
) -> Result<(SpannedValue, line_number), NmstateError> {
    let pos = YamlPosition::new(line_offset, 0, 0);

    if lines.len() == 1 {
        return Ok(yaml_value_to_spanned(
            serde_yaml::from_str(&lines[0])?,
            pos,
        ));
    } else {
        let mut ret_vec: Vec<SpannedValue> = Vec::new();
        let mut ret_map: HashMap<String, SpannedValue> = HashMap::new();
        for (line_number, line) in lines.iter().enumerate() {
            println!("HAHA {:?}", line);
            if line.is_empty() {
                continue;
            }
            let cur_line_number = line_number + line_offset;
            let (is_sequence, cur_indent) = get_indent(line);
            if cur_indent < indent {
                break;
            }
            // TODO: We should support JSON here also, like `: {`
            if line.ends_with(":") {
                if !is_sequence {
                    ret_map.insert(
                        line.trim_start()[..(line.len() - 1)].to_string(),
                        _deserialize_yaml_to_spanned_value(
                            lines[line_number + 1..],
                            cur_line_number + 1,
                        )?,
                    );
                } else {
                    ret_vec.insert(

                    todo!();
                }
            } else {

                match yaml_value_to_spanned(
                    serde_yaml::from_str(line)?,
                    YamlPosition::new(cur_line_number, 0, 0),
                ) {
                    SpannedValue::Sequence(pos, items) => {
                        for item in items {
                            ret_vec.push(item);
                        }
                    }
                    SpannedValue::Mapping(pos, mut items) => {
                        for (key, value) in items.drain() {
                            ret_map.insert(key, value);
                        }
                    }
                    n => {
                        println!(
                            "HAHA unexpected line {cur_line_number} {:?}",
                            line
                        );
                        log::error!(
                            "Got unexpected line {cur_line_number}: '{line}'"
                        );
                    }
                }
            }
        }
        if !ret_vec.is_empty() {
            Ok(SpannedValue::Sequence(pos, ret_vec))
        } else {
            Ok(SpannedValue::Mapping(pos, ret_map))
        }
    }
}

// Return bool(is_sequence), usize(space_count_including_`-`)
fn get_indent(line: &str) -> (bool, usize) {
    let is_sequence = line.trim_start().starts_with("-");
    for (c_number, c) in line.chars().enumerate() {
        if !['-', ' '].contains(&c) {
            return (is_sequence, c_number);
        }
    }
    (is_sequence, 0)
}

fn yaml_value_to_spanned(v: Value, pos: YamlPosition) -> SpannedValue {
    match v {
        Value::Null => SpannedValue::Null(pos),
        Value::Bool(i) => SpannedValue::Bool(pos, i),
        Value::Number(i) => SpannedValue::Number(pos, i),
        Value::String(i) => SpannedValue::String(pos, i),
        Value::Sequence(items) => {
            // TODO: each item should has its own position
            let mut spanned_items = Vec::new();
            for item in items {
                spanned_items.push(yaml_value_to_spanned(item, pos))
            }
            SpannedValue::Sequence(pos, spanned_items)
        }
        Value::Mapping(mut items) => {
            let mut spanned_map: HashMap<String, SpannedValue> = HashMap::new();
            let keys: Vec<String> = items
                .keys()
                .filter_map(|k| {
                    if let Some(s) = k.as_str() {
                        Some(s.to_string())
                    } else {
                        None
                    }
                })
                .collect();
            for key in keys {
                if let Some(v) = items.remove(&key) {
                    spanned_map.insert(key, yaml_value_to_spanned(v, pos));
                }
            }
            SpannedValue::Mapping(pos, spanned_map)
        }
        _ => {
            log::error!(
                "BUG: yaml_value_to_spanned() got unsupported YAML \
                format {:?}, treating as NULL",
                v
            );
            SpannedValue::Null(pos)
        }
    }
}

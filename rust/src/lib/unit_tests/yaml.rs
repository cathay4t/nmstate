// SPDX-License-Identifier: Apache-2.0

use crate::yaml::deserialize_yaml_to_spanned_value;

#[test]
fn test_deserialize_yaml_to_spanned() {
    let yaml_content = r#"---
interfaces:
- name: eth1
  type: ethernet
  state: up
  mtu: 1500
  ipv5:
    dhcp: false
    enabled: true
"#;

    let spanned = deserialize_yaml_to_spanned_value(yaml_content).unwrap();
    println!("HAHA test got {:?}", spanned);

    assert!(false);
}

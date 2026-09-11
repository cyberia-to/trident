use std::collections::BTreeMap;

use serde::Deserialize;

use super::{BundleCost, BundleFunction, ProgramBundle};

#[derive(Deserialize)]
struct WireBundle {
    name: String,
    version: String,
    target_vm: String,
    #[serde(default)]
    target_os: Option<String>,
    assembly: String,
    entry_point: String,
    #[serde(default)]
    functions: Vec<BundleFunction>,
    #[serde(default)]
    cost: BTreeMap<String, u64>,
    source_hash: String,
    #[serde(default)]
    reads_state: bool,
}

impl ProgramBundle {
    /// The legacy flat cost object remains readable; metadata survives transport.
    pub fn to_json(&self) -> String {
        let mut cost: BTreeMap<String, u64> = self
            .cost
            .table_names
            .iter()
            .cloned()
            .zip(self.cost.table_values.iter().copied())
            .collect();
        cost.insert("padded_height".into(), self.cost.padded_height);
        cost.insert(
            "estimated_proving_ns".into(),
            self.cost.estimated_proving_ns,
        );
        let mut value = serde_json::json!({
            "name": self.name, "version": self.version, "target_vm": self.target_vm,
            "target_os": self.target_os, "assembly": self.assembly,
            "entry_point": self.entry_point, "functions": self.functions,
            "cost": cost, "source_hash": self.source_hash,
        });
        if self.reads_state {
            value["reads_state"] = serde_json::Value::Bool(true);
        }
        value.to_string()
    }

    pub fn from_json(json: &str) -> Result<Self, String> {
        let mut wire: WireBundle = serde_json::from_str(json)
            .map_err(|error| format!("invalid program bundle: {error}"))?;
        let padded_height = wire.cost.remove("padded_height").unwrap_or(0);
        let estimated_proving_ns = wire.cost.remove("estimated_proving_ns").unwrap_or(0);
        let (table_names, table_values) = wire.cost.into_iter().unzip();
        Ok(Self {
            name: wire.name,
            version: wire.version,
            target_vm: wire.target_vm,
            target_os: wire.target_os,
            assembly: wire.assembly,
            entry_point: wire.entry_point,
            functions: wire.functions,
            cost: BundleCost {
                table_names,
                table_values,
                padded_height,
                estimated_proving_ns,
            },
            source_hash: wire.source_hash,
            reads_state: wire.reads_state,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_and_escaped_source_survive_transport() {
        let mut bundle = super::super::tests::sample_bundle();
        bundle.name = "quoted \"assembly\"\nλ".into();
        let restored = ProgramBundle::from_json(&bundle.to_json()).unwrap();
        assert_eq!(restored.functions, bundle.functions);
        assert_eq!(restored.name, bundle.name);
        assert_eq!(restored.assembly, bundle.assembly);
        let costs = |b: &ProgramBundle| {
            b.cost
                .table_names
                .iter()
                .zip(&b.cost.table_values)
                .map(|(n, v)| (n.clone(), *v))
                .collect::<BTreeMap<_, _>>()
        };
        assert_eq!(costs(&restored), costs(&bundle));
    }

    #[test]
    fn malformed_or_duplicate_identity_fields_are_rejected() {
        let json = super::super::tests::sample_bundle().to_json();
        assert!(ProgramBundle::from_json(&json[..json.len() - 1]).is_err());
        let duplicate = json.replacen('{', "{\"target_vm\":\"nox\",", 1);
        assert!(ProgramBundle::from_json(&duplicate).is_err());
        let wrong_type = json.replace("\"source_hash\":\"deadbeef\"", "\"source_hash\":null");
        assert!(ProgramBundle::from_json(&wrong_type).is_err());
    }
}

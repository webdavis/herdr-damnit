use herdr_damnit_domain::{Change, Oid, Op};
use serde::Deserialize;

use crate::wire::WireObject;

#[derive(Deserialize)]
pub(super) struct WireChange {
    pub(super) oid: String,
    op: String,
    #[serde(default)]
    subject: String,
    #[serde(default)]
    pub(super) fields: Vec<String>,
    #[serde(default, rename = "done")]
    pub(super) leaves_the_object_done: bool,
    #[serde(default)]
    pub(super) completed_at: Option<String>,
    #[serde(default)]
    after: Option<WireObject>,
}

pub(super) fn into_change(wire: WireChange) -> Change {
    let subject = match wire.subject.is_empty() {
        false => wire.subject,
        true => wire
            .after
            .as_ref()
            .map(|object| object.subject.clone())
            .unwrap_or_default(),
    };
    Change {
        oid: Oid::new(wire.oid),
        op: match wire.op.as_str() {
            "create" => Op::Create,
            "delete" => Op::Delete,
            _ => Op::Update,
        },
        subject,
        fields: wire.fields,
    }
}

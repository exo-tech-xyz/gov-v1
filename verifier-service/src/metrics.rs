use std::collections::HashMap;
use std::sync::Mutex;

use once_cell::sync::OnceCell;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum UploadOutcome {
    Success,
    BadRequest,
    Unauthorized,
    Internal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ProofKind {
    Vote,
    Stake,
}

pub struct Metrics {
    upload_total: HashMap<UploadOutcome, u64>,
    proofs_not_found_total: HashMap<ProofKind, u64>,
}

static METRICS: OnceCell<Mutex<Metrics>> = OnceCell::new();

fn get() -> &'static Mutex<Metrics> {
    METRICS.get_or_init(|| {
        Mutex::new(Metrics {
            upload_total: HashMap::new(),
            proofs_not_found_total: HashMap::new(),
        })
    })
}

pub fn record_upload_outcome(outcome: UploadOutcome) {
    let mut m = get().lock().expect("metrics mutex poisoned");
    *m.upload_total.entry(outcome).or_insert(0) += 1;
}

pub fn record_proofs_not_found(kind: ProofKind) {
    let mut m = get().lock().expect("metrics mutex poisoned");
    *m.proofs_not_found_total.entry(kind).or_insert(0) += 1;
}

pub fn snapshot_as_json() -> serde_json::Value {
    use serde_json::json;
    let m = get().lock().expect("metrics mutex poisoned");

    let uploads: Vec<serde_json::Value> = m
        .upload_total
        .iter()
        .map(|(outcome, count)| {
            json!({
                "outcome": match outcome {
                    UploadOutcome::Success => "success",
                    UploadOutcome::BadRequest => "bad_request",
                    UploadOutcome::Unauthorized => "unauthorized",
                    UploadOutcome::Internal => "internal",
                },
                "count": count
            })
        })
        .collect();

    let not_found: Vec<serde_json::Value> = m
        .proofs_not_found_total
        .iter()
        .map(|(kind, count)| {
            json!({
                "kind": match kind { ProofKind::Vote => "vote", ProofKind::Stake => "stake" },
                "count": count
            })
        })
        .collect();

    json!({
        "upload_total": uploads,
        "proofs_not_found_total": not_found,
    })
}

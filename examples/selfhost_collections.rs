//! Deterministic source/input/expected fixtures for the installed Joy CLI gate.
//! Expected trees come from the SH0 reference model; no guest execution here.
use clap::Parser;
use nox::{artifact, Order, Reduction};
use std::path::{Path, PathBuf};
#[path = "selfhost_data/model.rs"]
#[allow(dead_code)]
mod model;
type Arena = Reduction<16384>;

#[derive(Parser)]
struct Args {
    #[arg(long)]
    output_dir: PathBuf,
}

fn encode(ar: &Arena, root: Order) -> Result<Vec<u8>, String> {
    artifact::encode(ar, root, trident::RAW_ARTIFACT_LIMITS).map_err(|e| format!("{e:?}"))
}

fn fixture(
    dir: &Path,
    name: &str,
    source: &str,
    input: Vec<u8>,
    expected: Option<Vec<u8>>,
) -> Result<serde_json::Value, String> {
    let base = dir.join(name);
    std::fs::create_dir_all(&base).map_err(|e| e.to_string())?;
    std::fs::write(base.join("main.tri"), source).map_err(|e| e.to_string())?;
    std::fs::write(base.join("input.dag"), input).map_err(|e| e.to_string())?;
    if let Some(bytes) = &expected {
        std::fs::write(base.join("expected.dag"), bytes).map_err(|e| e.to_string())?;
    }
    Ok(serde_json::json!({"name":name,"should_succeed":expected.is_some()}))
}

fn generate(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut ar = Arena::new();
    let mut fixtures = Vec::new();
    let source = "program byte_probe\nuse vm.nox.noun\nuse std.nox.bytes\nfn main(input: Noun) -> Noun {\nlet b = bytes.from_noun(noun.head(input), as_u32(4096), as_u32(noun.as_field(noun.tail(input))))\nlet changed = bytes.set(b, as_u32(3), as_u32(0))\nbytes.to_noun(bytes.push(changed, as_u32(255), as_u32(4096)))\n}\n";
    let data: Vec<_> = (0..517).map(|i| (i * 37) as u8).collect();
    let bytes = model::Bytes::from_slice(&mut ar, &data, 4096).map_err(|e| format!("{e:?}"))?;
    let root = bytes.encode(&mut ar).map_err(|e| format!("{e:?}"))?;
    let mut remaining = 20000;
    model::Bytes::decode_budget(&mut ar, root, 4096, &mut remaining)
        .map_err(|e| format!("{e:?}"))?;
    let visits = 20000 - remaining;
    let quota = model::atom(&mut ar, u64::from(visits)).map_err(|e| format!("{e:?}"))?;
    let input = model::pair(&mut ar, root, quota).map_err(|e| format!("{e:?}"))?;
    let expected = bytes
        .set(&mut ar, 3, 0)
        .and_then(|b| b.push(&mut ar, 255, 4096))
        .and_then(|b| b.encode(&mut ar))
        .map_err(|e| format!("{e:?}"))?;
    fixtures.push(fixture(
        dir,
        "bytes-517",
        source,
        encode(&ar, input)?,
        Some(encode(&ar, expected)?),
    )?);
    let low = model::atom(&mut ar, u64::from(visits - 1)).map_err(|e| format!("{e:?}"))?;
    let input = model::pair(&mut ar, root, low).map_err(|e| format!("{e:?}"))?;
    fixtures.push(fixture(
        dir,
        "bytes-visits-short",
        source,
        encode(&ar, input)?,
        None,
    )?);

    // Keep a valid operation length so a later bounds check cannot mask a
    // broken padding validator: relabel a 518-byte tree as 517 bytes.
    let mut extended = data.clone();
    extended.push(1);
    let extended = model::Bytes::from_slice(&mut ar, &extended, 4096)
        .and_then(|b| b.encode(&mut ar))
        .map_err(|e| format!("{e:?}"))?;
    let extended_body = ar.tail(extended).ok_or("missing bytes body")?;
    let word_tree = ar.tail(extended_body).ok_or("missing word tree")?;
    let length = model::atom(&mut ar, 517).map_err(|e| format!("{e:?}"))?;
    let one = model::atom(&mut ar, 1).map_err(|e| format!("{e:?}"))?;
    let body = model::pair(&mut ar, length, word_tree).map_err(|e| format!("{e:?}"))?;
    let tag = model::atom(&mut ar, model::BYTES).map_err(|e| format!("{e:?}"))?;
    let bad = model::pair(&mut ar, tag, body).map_err(|e| format!("{e:?}"))?;
    let input = model::pair(&mut ar, bad, quota).map_err(|e| format!("{e:?}"))?;
    fixtures.push(fixture(
        dir,
        "bytes-bad-padding",
        source,
        encode(&ar, input)?,
        None,
    )?);

    let source = "program seq_probe\nuse vm.nox.noun\nuse std.nox.seq\nfn main(input: Noun) -> Noun {\nlet s=seq.from_noun(input,as_u32(1000),as_u32(10000))\nlet changed=seq.set(s,as_u32(64),noun.pair(noun.atom(8),noun.atom(9)))\nseq.to_noun(seq.push(changed,noun.pair(noun.atom(7),noun.atom(9)),as_u32(1000)))\n}\n";
    let values = (0..129)
        .map(|i| {
            let n = model::atom(&mut ar, i)?;
            model::pair(&mut ar, n, one)
        })
        .collect::<model::Result<Vec<_>>>()
        .map_err(|e| format!("{e:?}"))?;
    let sequence = model::Seq::from_values(&mut ar, &values, 1000).map_err(|e| format!("{e:?}"))?;
    let input = sequence.encode(&mut ar).map_err(|e| format!("{e:?}"))?;
    let seven = model::atom(&mut ar, 7).map_err(|e| format!("{e:?}"))?;
    let eight = model::atom(&mut ar, 8).map_err(|e| format!("{e:?}"))?;
    let nine = model::atom(&mut ar, 9).map_err(|e| format!("{e:?}"))?;
    let replace = model::pair(&mut ar, eight, nine).map_err(|e| format!("{e:?}"))?;
    let append = model::pair(&mut ar, seven, nine).map_err(|e| format!("{e:?}"))?;
    let expected = sequence
        .set(&mut ar, 64, replace)
        .and_then(|s| s.push(&mut ar, append, 1000))
        .and_then(|s| s.encode(&mut ar))
        .map_err(|e| format!("{e:?}"))?;
    fixtures.push(fixture(
        dir,
        "seq-129",
        source,
        encode(&ar, input)?,
        Some(encode(&ar, expected)?),
    )?);
    let json =
        serde_json::to_vec_pretty(&serde_json::json!({"fixtures":fixtures,"bytes_visits":visits}))
            .map_err(|e| e.to_string())?;
    std::fs::write(dir.join("fixtures.json"), json).map_err(|e| e.to_string())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || generate(&args.output_dir))?
        .join()
        .map_err(|_| "fixture worker panicked")??;
    Ok(())
}

//! Diagnóstico conservador fora da TCB. Nenhuma operação remove/escreve objetos.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use logos_canon::{Value, from_canon};
use logos_crypto::{Domain, Hash, PublicKey, tags};
use logos_ledger::{Cas, Limits, ledger_references};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Address {
    domain: usize,
    digest: [u8; 32],
}

struct At<const N: usize>;
impl<const N: usize> Domain for At<N> {
    const TAG: &'static [u8] = tags::RESERVED[N].1;
}

fn read(cas: &Cas, address: Address) -> Result<Vec<u8>> {
    macro_rules! domains {
        ($($n:literal),*) => { match address.domain {
            $($n => cas.get::<At<$n>>(&Hash::from_bytes(address.digest))?,)*
            _ => return Err("unknown CAS domain".into()),
        } };
    }
    domains!(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17)
        .ok_or_else(|| format!("CAS object disappeared: {}", label(address)).into())
}
fn directory(domain: usize) -> String {
    format!("{}-v1", tags::RESERVED[domain].0)
}
fn label(address: Address) -> String {
    let hex: String = address.digest.iter().map(|b| format!("{b:02x}")).collect();
    format!("{}/{}/{}", directory(address.domain), &hex[..2], &hex[2..])
}
fn lower_hex(name: &str, len: usize) -> bool {
    name.len() == len
        && name
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn entries(path: &Path) -> Result<Vec<fs::DirEntry>> {
    let mut result: Vec<_> = fs::read_dir(path)?.collect::<std::io::Result<_>>()?;
    result.sort_by_key(fs::DirEntry::file_name);
    Ok(result)
}
fn name(entry: &fs::DirEntry) -> Result<String> {
    entry
        .file_name()
        .into_string()
        .map_err(|_| "non-UTF8 CAS name".into())
}
fn need_type(entry: &fs::DirEntry, directory: bool) -> Result<()> {
    let ty = entry.file_type()?; // não segue symlinks.
    if if directory {
        !ty.is_dir()
    } else {
        !ty.is_file()
    } {
        return Err(format!("not a regular CAS path: {:?}", entry.path()).into());
    }
    Ok(())
}
fn temporary(name: &str) -> bool {
    match name.split('.').collect::<Vec<_>>().as_slice() {
        ["", "tmp", pid, sequence] => {
            !pid.is_empty()
                && !sequence.is_empty()
                && pid.bytes().all(|b| b.is_ascii_digit())
                && sequence.bytes().all(|b| b.is_ascii_digit())
                && pid.parse::<u32>().is_ok()
                && sequence.parse::<u64>().is_ok()
        }
        _ => false,
    }
}
fn inventory(root: &Path) -> Result<(BTreeSet<Address>, Vec<PathBuf>)> {
    let mut objects = BTreeSet::new();
    let mut temporaries = Vec::new();
    for domain in entries(root)? {
        need_type(&domain, true)?;
        let domain_name = name(&domain)?;
        let index = (0..tags::RESERVED.len())
            .find(|&i| domain_name == directory(i))
            .ok_or("unknown CAS directory")?;
        for shard in entries(&domain.path())? {
            need_type(&shard, true)?;
            let prefix = name(&shard)?;
            if !lower_hex(&prefix, 2) {
                return Err("invalid CAS shard".into());
            }
            for object in entries(&shard.path())? {
                need_type(&object, false)?;
                let suffix = name(&object)?;
                if temporary(&suffix) {
                    temporaries.push(object.path());
                    continue;
                }
                if !lower_hex(&suffix, 62) {
                    return Err("invalid CAS object name".into());
                }
                let hex = prefix.clone() + &suffix;
                let mut digest = [0; 32];
                for (index, pair) in hex.as_bytes().as_chunks::<2>().0.iter().enumerate() {
                    digest[index] = u8::from_str_radix(std::str::from_utf8(pair)?, 16)?;
                }
                objects.insert(Address {
                    domain: index,
                    digest,
                });
            }
        }
    }
    Ok((objects, temporaries))
}
fn links(bytes: &[u8]) -> Result<BTreeSet<[u8; 32]>> {
    let value: Value = from_canon(bytes)?;
    let mut pending = vec![&value];
    let mut result = BTreeSet::new();
    while let Some(value) = pending.pop() {
        match value {
            Value::Bytes(bytes) if bytes.len() == 32 => {
                result.insert(bytes.as_slice().try_into()?);
            }
            Value::Array(array) => pending.extend(array.items()),
            Value::Map(map) => {
                for (key, value) in map.entries() {
                    pending.extend([key, value]);
                }
            }
            _ => {}
        }
    }
    Ok(result)
}

pub(crate) fn run(directory_path: &Path, key: &PublicKey) -> Result<()> {
    let references = ledger_references(directory_path, key)?; // lock mantido até o fim.
    let root = directory_path.join("cas");
    let cas = Cas::open_existing(&root, Limits::default())?;
    let (objects, temporaries) = inventory(&root)?;
    let mut by_digest: BTreeMap<[u8; 32], Vec<Address>> = BTreeMap::new();
    for &address in &objects {
        read(&cas, address)?; // integridade inclusive de candidatos órfãos.
        by_digest.entry(address.digest).or_default().push(address);
    }
    let mut pending = Vec::new();
    for reference in &references.objects {
        let domain = (0..tags::RESERVED.len())
            .find(|&i| directory(i) == reference.directory())
            .ok_or("unknown reference domain")?;
        let address = Address {
            domain,
            digest: reference.digest(),
        };
        if !objects.contains(&address) {
            return Err(format!("missing CAS object {}", label(address)).into());
        }
        pending.push(address);
    }
    let mut retained = BTreeSet::new();
    while let Some(address) = pending.pop() {
        if !retained.insert(address) {
            continue;
        }
        if tags::RESERVED[address.domain].2 == "raw" {
            continue;
        }
        for digest in links(&read(&cas, address)?)? {
            if let Some(matches) = by_digest.get(&digest) {
                pending.extend(matches);
            }
        }
    }
    // Só imprimir após conferir tudo; falha não emite lista parcial de órfãos.
    let orphans: Vec<_> = objects.difference(&retained).copied().collect();
    println!(
        "LEDGER_GC_DRY_RUN events={} objects={} referenced={} retained={} orphan_candidates={} temporaries={} head={}",
        references.head.map_or(0, |h| h.seq + 1),
        objects.len(),
        references.objects.len(),
        retained.len(),
        orphans.len(),
        temporaries.len(),
        references
            .head
            .map_or_else(|| "empty".into(), |h| h.hash.to_hex())
    );
    for address in orphans {
        println!("ORPHAN {}", label(address));
    }
    for temporary in temporaries {
        println!("TEMP {:?}", temporary.strip_prefix(&root)?);
    }
    Ok(())
}

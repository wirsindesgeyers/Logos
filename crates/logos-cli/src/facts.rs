//! Confiança, fatos e revogação de emissor (6.1, 6.2, 6.4) com o kernel parado.
//!
//! Mesma autoridade do `logos-kernel`: a chave do kernel é a do admin, como na retratação
//! (5.9). A API da TCB decide o que é aceito; esta CLI só a chama e imprime o resultado.

use std::{ffi::OsString, fs, path::Path};

use logos_air::Vocabulary;
use logos_canon::Text;
use logos_crypto::{FileSigner, Hash, KeyId, PublicKey, SigAlg, Signer};
use logos_kernel::facts::{publish_trust, register, revoke_issuer};
use logos_ledger::{EventBody, Fact, Ledger, TrustConfig, TrustRoot};

type Error = Box<dyn std::error::Error>;

/// Teto de leitura de arquivos de fato e vocabulário (o CAS aplica o seu).
const MAX_FILE: u64 = 4 * 1024 * 1024;

fn read_capped(path: &Path) -> Result<Vec<u8>, Error> {
    let meta = fs::symlink_metadata(path)?;
    if !meta.is_file() || meta.len() > MAX_FILE {
        return Err(format!("{} is not a regular file up to 4 MiB", path.display()).into());
    }
    Ok(fs::read(path)?)
}

fn hex32(text: &str, what: &str) -> Result<[u8; 32], Error> {
    if text.len() != 64 || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(format!("{what} must contain exactly 64 hex digits").into());
    }
    let mut bytes = [0; 32];
    for (result, pair) in bytes.iter_mut().zip(text.as_bytes().as_chunks::<2>().0) {
        *result = u8::from_str_radix(std::str::from_utf8(pair)?, 16)?;
    }
    Ok(bytes)
}

/// Flags `--nome valor`; `--root` pode repetir. Nada além disso é aceito.
struct Flags(Vec<(String, String)>);

impl Flags {
    fn parse(args: &[OsString]) -> Result<Self, Error> {
        let mut pairs = Vec::new();
        let mut it = args.iter();
        while let Some(flag) = it.next() {
            let flag = flag.to_str().ok_or("flags must be UTF-8")?;
            let value = it
                .next()
                .and_then(|v| v.to_str())
                .ok_or(format!("{flag} needs a UTF-8 value"))?;
            if !flag.starts_with("--") {
                return Err(format!("unexpected argument {flag}").into());
            }
            pairs.push((flag.to_owned(), value.to_owned()));
        }
        Ok(Self(pairs))
    }

    fn all(&self, name: &str) -> Vec<&str> {
        self.0
            .iter()
            .filter(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
            .collect()
    }

    fn one(&self, name: &str) -> Result<&str, Error> {
        match self.all(name).as_slice() {
            [value] => Ok(value),
            _ => Err(format!("exactly one {name} is required").into()),
        }
    }

    fn only(&self, allowed: &[&str]) -> Result<(), Error> {
        for (name, _) in &self.0 {
            if !allowed.contains(&name.as_str()) {
                return Err(format!("unknown flag {name}").into());
            }
        }
        Ok(())
    }
}

/// `root` = `PUBKEY_HEX:NOME:TOPICO[,TOPICO…]`.
fn parse_root(text: &str) -> Result<TrustRoot, Error> {
    let mut parts = text.splitn(3, ':');
    let (Some(key), Some(name), Some(topics)) = (parts.next(), parts.next(), parts.next()) else {
        return Err("--root must be PUBKEY_HEX:NAME:TOPIC[,TOPIC...]".into());
    };
    let mut topics = topics
        .split(',')
        .map(Text::new)
        .collect::<Result<Vec<_>, _>>()?;
    topics.sort();
    Ok(TrustRoot {
        issuer: PublicKey::new(SigAlg::Ed25519, &hex32(key, "issuer public key")?)?,
        name: Text::new(name)?,
        topics,
        level: 1,
    })
}

/// Tópicos que o usuário atesta (6.5): autorizar ações, confirmar e assumir premissas.
const USER_TOPICS: [&str; 3] = ["assume", "authorize", "confirm"];

/// Configuração de confiança vigente (a de maior versão publicada), se houver.
fn current_trust(ledger: &Ledger<'_, FileSigner>) -> Result<Option<TrustConfig>, Error> {
    let mut best: Option<TrustConfig> = None;
    let mut from = 0;
    loop {
        let page = ledger.events_from(from, 256)?;
        let Some(last) = page.last().map(|e| e.seq) else {
            return Ok(best);
        };
        for event in &page {
            if let EventBody::TrustConfigPublished { config, .. } = &event.body {
                let bytes = ledger
                    .cas()
                    .get(config)?
                    .ok_or("trust config missing from CAS")?;
                let decoded: TrustConfig = logos_canon::from_canon(&bytes)?;
                if best.as_ref().is_none_or(|b| decoded.version > b.version) {
                    best = Some(decoded);
                }
            }
        }
        from = last + 1;
    }
}

pub fn run(command: &str, directory: &Path, rest: &[OsString]) -> Result<(), Error> {
    let flags = Flags::parse(rest)?;
    match command {
        "trust-publish" => {
            flags.only(&["--key", "--version", "--root"])?;
            let signer = FileSigner::load(Path::new(flags.one("--key")?))?;
            let mut roots = flags
                .all("--root")
                .into_iter()
                .map(parse_root)
                .collect::<Result<Vec<_>, _>>()?;
            roots.sort_by_key(|r| r.issuer.key_id().map(|k| *k.as_bytes()).unwrap_or_default());
            let config = TrustConfig::new(flags.one("--version")?.parse()?, roots)?;
            let mut ledger = Ledger::open(directory, &signer)?;
            let hash = ledger
                .cas()
                .put::<TrustConfig>(&logos_canon::to_canon(&config)?)?;
            let event = publish_trust(&mut ledger, signer.public_key(), &signer, hash)?;
            println!(
                "{{\"config\":\"{}\",\"event\":\"{}\"}}",
                hash.to_hex(),
                event.to_hex()
            );
        }
        "fact-register" => {
            flags.only(&["--key", "--fact", "--vocabulary"])?;
            let signer = FileSigner::load(Path::new(flags.one("--key")?))?;
            let fact_bytes = read_capped(Path::new(flags.one("--fact")?))?;
            let vocabulary_bytes = read_capped(Path::new(flags.one("--vocabulary")?))?;
            let mut ledger = Ledger::open(directory, &signer)?;
            ledger.cas().put::<Vocabulary>(&vocabulary_bytes)?;
            let fact: Hash<Fact> = ledger.cas().put::<Fact>(&fact_bytes)?;
            let event = register(&mut ledger, fact)?;
            println!(
                "{{\"fact\":\"{}\",\"event\":\"{}\"}}",
                fact.to_hex(),
                event.to_hex()
            );
        }
        "user-trust" => {
            flags.only(&["--key", "--user-public"])?;
            let signer = FileSigner::load(Path::new(flags.one("--key")?))?;
            let user = PublicKey::new(
                SigAlg::Ed25519,
                &hex32(flags.one("--user-public")?, "user public key")?,
            )?;
            let mut ledger = Ledger::open(directory, &signer)?;
            let current = current_trust(&ledger)?;
            let version = current.as_ref().map_or(1, |c| c.version + 1);
            let mut roots = current.map(|c| c.roots).unwrap_or_default();
            let user_id = user.key_id()?;
            if roots
                .iter()
                .any(|r| r.issuer.key_id().is_ok_and(|id| id == user_id))
            {
                return Err("user key is already a trust root".into());
            }
            roots.push(TrustRoot {
                issuer: user,
                name: Text::new("user")?,
                topics: USER_TOPICS
                    .iter()
                    .map(|t| Text::new(*t))
                    .collect::<Result<_, _>>()?,
                level: 1,
            });
            roots.sort_by_key(|r| r.issuer.key_id().map(|k| *k.as_bytes()).unwrap_or_default());
            let config = TrustConfig::new(version, roots)?;
            let hash = ledger
                .cas()
                .put::<TrustConfig>(&logos_canon::to_canon(&config)?)?;
            let event = publish_trust(&mut ledger, signer.public_key(), &signer, hash)?;
            println!(
                "{{\"config\":\"{}\",\"event\":\"{}\",\"version\":{version}}}",
                hash.to_hex(),
                event.to_hex()
            );
        }
        "revoke-issuer" => {
            flags.only(&["--key", "--issuer", "--since"])?;
            let signer = FileSigner::load(Path::new(flags.one("--key")?))?;
            let issuer = KeyId::from_bytes(hex32(flags.one("--issuer")?, "issuer key id")?);
            let mut ledger = Ledger::open(directory, &signer)?;
            let revocation = revoke_issuer(
                &mut ledger,
                signer.public_key(),
                &signer,
                issuer,
                flags.one("--since")?.parse()?,
            )?;
            let invalidated: Vec<String> = revocation
                .invalidated
                .iter()
                .map(|h| format!("\"{}\"", h.to_hex()))
                .collect();
            println!(
                "{{\"revoked\":\"{}\",\"invalidated\":[{}]}}",
                revocation.revoked.to_hex(),
                invalidated.join(",")
            );
        }
        _ => return Err("unknown command".into()),
    }
    Ok(())
}

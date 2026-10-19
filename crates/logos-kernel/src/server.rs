//! Servidor Linux: UID do SO, frames limitados e handlers sem autoridade implícita.

use crate::{
    TransportError as Error,
    staging::Staging,
    wire::{self, Message, Query, Request},
};
use logos_air::{Formula, Vocabulary};
use logos_canon::{Array, Canon, Text, Value};
use logos_crypto::{Hash, Signer};
use logos_ledger::{Certificate, EventProposal, Ledger, ModelObject};
use logos_obligation::{EntryKind, Obligation};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::{
    fs::{MetadataExt, PermissionsExt},
    net::{UnixListener, UnixStream},
};
use std::path::Path;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

/// Identidade observada pelo SO; não é campo fornecido pelo cliente.
#[derive(Debug, Clone, Copy)]
pub struct Peer {
    uid: u32,
    gid: u32,
    pid: u32,
}
impl Peer {
    /// UID do peer.
    pub fn uid(&self) -> u32 {
        self.uid
    }
    /// GID primário do peer.
    pub fn gid(&self) -> u32 {
        self.gid
    }
    /// PID do processo no momento da conexão.
    pub fn pid(&self) -> u32 {
        self.pid
    }
}

/// Limites transitórios por conexão; nenhum é veredicto/orçamento de checker.
#[derive(Clone, Copy, Debug)]
pub struct ConnectionLimits {
    /// Payload máximo de requisição, limitado pelo wire v1.
    pub frame_bytes: usize,
    /// Frames aceitos por segundo.
    pub frames_per_second: u32,
    /// Total máximo de frames.
    pub frames: u64,
    /// Quota conjunta de entrada e saída.
    pub bytes: u64,
    /// Bytes importados do CAS/staging.
    pub import_bytes: u64,
    /// Prazo absoluto de I/O de cada frame.
    pub frame_timeout: Duration,
    /// Prazo da assinatura de eventos.
    pub subscription_timeout: Duration,
    /// Conexões concorrentes, todas do UID configurado.
    pub connections: usize,
}
impl Default for ConnectionLimits {
    fn default() -> Self {
        Self {
            frame_bytes: wire::MAX_REQUEST_BYTES,
            frames_per_second: 32,
            frames: 1024,
            bytes: 64 * 1024 * 1024,
            import_bytes: 128 * 1024 * 1024,
            frame_timeout: Duration::from_secs(5),
            subscription_timeout: Duration::from_secs(5),
            connections: 8,
        }
    }
}
impl ConnectionLimits {
    fn validate(&self) -> Result<(), Error> {
        if self.frame_bytes == 0
            || self.frame_bytes > wire::MAX_REQUEST_BYTES
            || self.frames_per_second == 0
            || self.frames == 0
            || self.bytes < 4
            || self.frame_timeout.is_zero()
            || self.frame_timeout > Duration::from_secs(60)
            || self.subscription_timeout.is_zero()
            || self.subscription_timeout > Duration::from_secs(60)
            || !(1..=8).contains(&self.connections)
        {
            return Err(Error::Invalid("connection limits"));
        }
        Ok(())
    }
}

/// Handlers confiados pelo operador. Estado lógico deve ficar no Ledger.
/// Implementações padrão recusam mutações das etapas ainda não implementadas.
pub trait Handlers<S: Signer>: Sync {
    /// PROPOSE5.3; o transporte não cria obrigação.
    fn propose(
        &self,
        _ledger: &mut Ledger<'_, S>,
        _peer: &Peer,
        _proposal: EventProposal,
    ) -> Result<Value, Error> {
        Err(Error::Unavailable)
    }
    /// CERTIFY5.4; referência já conferida/importada, sem aceite de prova.
    fn certify(
        &self,
        _ledger: &mut Ledger<'_, S>,
        _peer: &Peer,
        _obligation: Hash<Obligation>,
        _format: Text,
        _certificate: Hash<Certificate>,
    ) -> Result<Value, Error> {
        Err(Error::Unavailable)
    }
    /// REFUTE5.4; importação não confere contramodelo.
    fn refute(
        &self,
        _ledger: &mut Ledger<'_, S>,
        _peer: &Peer,
        _obligation: Hash<Obligation>,
        _model: Hash<ModelObject>,
    ) -> Result<Value, Error> {
        Err(Error::Unavailable)
    }
    /// WITNESS5.5.
    fn witness(
        &self,
        _ledger: &mut Ledger<'_, S>,
        _peer: &Peer,
        _obligation: Hash<Obligation>,
        _model: Hash<ModelObject>,
    ) -> Result<Value, Error> {
        Err(Error::Unavailable)
    }
    /// ASSUME5.7; identidade do peer não é assinatura de usuário.
    fn assume(
        &self,
        _ledger: &mut Ledger<'_, S>,
        _peer: &Peer,
        _vocabulary: Hash<Vocabulary>,
        _formula: Formula,
        _scope: Text,
    ) -> Result<Value, Error> {
        Err(Error::Unavailable)
    }
    /// Query somente de leitura; erro de índice não vira ausência.
    fn query(
        &self,
        ledger: &mut Ledger<'_, S>,
        _peer: &Peer,
        query: Query,
    ) -> Result<Value, Error> {
        match query {
            Query::Head => match ledger.head()? {
                Some(h) => array(vec![
                    h.seq.to_value()?,
                    h.epoch.to_value()?,
                    h.hash.to_value()?,
                ]),
                None => array(vec![]),
            },
            Query::Status(reference) => {
                let index = ledger.live_index()?;
                match index.status(reference)? {
                    None => array(vec![]),
                    Some(status) => {
                        let kind = match status.kind {
                            EntryKind::Fact => 0,
                            EntryKind::Policy => 1,
                            EntryKind::Verified => 2,
                            EntryKind::Assumption => 3,
                            EntryKind::Pending => 4,
                            EntryKind::Refuted => 5,
                            EntryKind::Unknown => 6,
                            EntryKind::Rejected => 7,
                            EntryKind::Invalidated => 8,
                            EntryKind::Proposal => 9,
                        };
                        array(vec![
                            Value::Unsigned(kind),
                            status.epoch.to_value()?,
                            status.retracted.to_value()?,
                        ])
                    }
                }
            }
        }
    }
    /// Página autenticada de eventos; o servidor libera o lock antes de transmitir.
    fn subscribe(
        &self,
        ledger: &mut Ledger<'_, S>,
        _peer: &Peer,
        from_seq: u64,
        limit: usize,
    ) -> Result<Vec<logos_ledger::Event>, Error> {
        Ok(ledger.events_from(from_seq, limit)?)
    }
}

/// Handlers de leitura reais e mutações explicitamente indisponíveis.
pub struct ReadOnlyHandlers;
impl<S: Signer> Handlers<S> for ReadOnlyHandlers {}

fn array(values: Vec<Value>) -> Result<Value, Error> {
    Ok(Value::Array(Array::new(values)?))
}
fn timeout() -> Error {
    Error::Io(std::io::Error::new(
        std::io::ErrorKind::TimedOut,
        "absolute frame deadline",
    ))
}
fn remaining(deadline: Instant) -> Result<Duration, Error> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|d| !d.is_zero())
        .ok_or_else(timeout)
}
fn read_exact(
    stream: &mut UnixStream,
    buffer: &mut [u8],
    deadline: Instant,
    allow_eof: bool,
) -> Result<bool, Error> {
    let mut offset = 0;
    while offset < buffer.len() {
        stream.set_read_timeout(Some(remaining(deadline)?))?;
        match stream.read(&mut buffer[offset..]) {
            Ok(0) if offset == 0 && allow_eof => return Ok(false),
            Ok(0) => {
                return Err(Error::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "partial frame",
                )));
            }
            Ok(n) => offset += n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(true)
}

struct Budget {
    bytes: u64,
    imports: u64,
    frames: u64,
    window: Instant,
    count: u32,
}
impl Budget {
    fn charge(&mut self, n: usize) -> Result<(), Error> {
        self.bytes = self
            .bytes
            .checked_sub(n as u64)
            .ok_or(Error::Limit("connection bytes"))?;
        Ok(())
    }
    fn frame(&mut self, limits: &ConnectionLimits) -> Result<(), Error> {
        self.frames = self
            .frames
            .checked_sub(1)
            .ok_or(Error::Limit("connection frames"))?;
        if self.window.elapsed() >= Duration::from_secs(1) {
            self.window = Instant::now();
            self.count = 0;
        }
        if self.count >= limits.frames_per_second {
            return Err(Error::Limit("frame rate"));
        }
        self.count += 1;
        Ok(())
    }
}
fn write_frame(
    stream: &mut UnixStream,
    payload: &[u8],
    budget: &mut Budget,
    limits: &ConnectionLimits,
) -> Result<(), Error> {
    write_frame_until(
        stream,
        payload,
        budget,
        Instant::now() + limits.frame_timeout,
    )
}

fn write_frame_until(
    stream: &mut UnixStream,
    payload: &[u8],
    budget: &mut Budget,
    deadline: Instant,
) -> Result<(), Error> {
    write_frame_io(stream, payload, budget, Some(deadline))
}

fn write_frame_io(
    stream: &mut UnixStream,
    payload: &[u8],
    budget: &mut Budget,
    deadline: Option<Instant>,
) -> Result<(), Error> {
    if payload.len() > wire::MAX_RESPONSE_BYTES {
        return Err(Error::Limit("response frame"));
    }
    budget.charge(payload.len() + 4)?;
    let length = u32::try_from(payload.len()).map_err(|_| Error::Limit("response length"))?;
    let prefix = length.to_be_bytes();
    for mut bytes in [&prefix[..], payload] {
        while !bytes.is_empty() {
            if let Some(deadline) = deadline {
                stream.set_write_timeout(Some(remaining(deadline)?))?;
            }
            match stream.write(bytes) {
                Ok(0) => {
                    return Err(Error::Io(std::io::Error::new(
                        std::io::ErrorKind::WriteZero,
                        "frame write",
                    )));
                }
                Ok(n) => bytes = &bytes[n..],
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e.into()),
            }
        }
    }
    Ok(())
}
fn authenticate(stream: &UnixStream, allowed_uid: u32) -> Result<Peer, Error> {
    let cred = rustix::net::sockopt::socket_peercred(stream).map_err(std::io::Error::from)?;
    if cred.uid.as_raw() != allowed_uid {
        return Err(Error::Unauthorized);
    }
    Ok(Peer {
        uid: cred.uid.as_raw(),
        gid: cred.gid.as_raw(),
        pid: cred.pid.as_raw_nonzero().get() as u32,
    })
}

/// Listener Linux para um principal UID configurado; não substitui caminho existente.
pub struct Server {
    listener: UnixListener,
    _directory: File,
    uid: u32,
    limits: ConnectionLimits,
}
impl Server {
    /// Cria socket em pai controlado pelo kernel. O operador provisiona grupos/ACLs.
    /// # Errors
    /// Configuração, pai/UID/mode inválidos ou bind/permissões falhos.
    pub fn bind(path: &Path, uid: u32, limits: ConnectionLimits) -> Result<Self, Error> {
        limits.validate()?;
        let parent = path.parent().ok_or(Error::Invalid("socket parent"))?;
        let name = path.file_name().ok_or(Error::Invalid("socket name"))?;
        let fd = rustix::fs::open(
            parent,
            rustix::fs::OFlags::RDONLY
                | rustix::fs::OFlags::DIRECTORY
                | rustix::fs::OFlags::NOFOLLOW
                | rustix::fs::OFlags::CLOEXEC,
            rustix::fs::Mode::empty(),
        )
        .map_err(std::io::Error::from)?;
        let directory = File::from(fd);
        let meta = directory.metadata()?;
        if meta.uid() != rustix::process::geteuid().as_raw() || meta.mode() & 0o022 != 0 {
            return Err(Error::Invalid("socket parent owner/mode"));
        }
        let anchored = Path::new("/proc/self/fd")
            .join(directory.as_raw_fd().to_string())
            .join(name);
        let listener = UnixListener::bind(&anchored)?;
        fs::set_permissions(&anchored, fs::Permissions::from_mode(0o660))?;
        listener.set_nonblocking(true)?;
        Ok(Self {
            listener,
            _directory: directory,
            uid,
            limits,
        })
    }

    /// Serve conexões do mesmo UID; compartilha somente o writer sob Mutex.
    /// # Errors
    /// Erro do listener ou panic de worker. Erros de cliente são registrados.
    pub fn serve<S: Signer + Sync, H: Handlers<S>>(
        &self,
        ledger: &mut Ledger<'_, S>,
        handlers: &H,
        staging: &Staging,
        stop: &AtomicBool,
    ) -> Result<(), Error> {
        let writer = Mutex::new(ledger);
        std::thread::scope(|scope| {
            let mut workers: Vec<std::thread::ScopedJoinHandle<'_, Result<(), Error>>> = Vec::new();
            while !stop.load(Ordering::Acquire) {
                for i in (0..workers.len()).rev() {
                    if workers[i].is_finished() {
                        match workers.swap_remove(i).join() {
                            Ok(Ok(())) => {}
                            Ok(Err(e)) => eprintln!("kernel connection: {e}"),
                            Err(_) => return Err(Error::Invalid("connection worker panic")),
                        }
                    }
                }
                match self.listener.accept() {
                    Ok((stream, _)) => {
                        if workers.len() >= self.limits.connections {
                            stream.shutdown(std::net::Shutdown::Both)?;
                            continue;
                        }
                        let writer = &writer;
                        workers.push(
                            scope.spawn(move || {
                                self.serve_stream(stream, writer, handlers, staging)
                            }),
                        );
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(10))
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                    Err(e) => return Err(e.into()),
                }
            }
            for worker in workers {
                match worker.join() {
                    Ok(Ok(())) => {}
                    Ok(Err(e)) => eprintln!("kernel connection: {e}"),
                    Err(_) => return Err(Error::Invalid("connection worker panic")),
                }
            }
            Ok(())
        })
    }

    /// Serve um stream já conectado, autenticando antes do primeiro byte.
    /// Também permite testes de pares Unix reais, sem um sink de persistência falso.
    /// # Errors
    /// UID, framing, limites, I/O ou handler/armazenamento inválidos.
    pub fn serve_stream<S: Signer, H: Handlers<S>>(
        &self,
        mut stream: UnixStream,
        writer: &Mutex<&mut Ledger<'_, S>>,
        handlers: &H,
        staging: &Staging,
    ) -> Result<(), Error> {
        let peer = authenticate(&stream, self.uid)?;
        let mut budget = Budget {
            bytes: self.limits.bytes,
            imports: self.limits.import_bytes,
            frames: self.limits.frames,
            window: Instant::now(),
            count: 0,
        };
        loop {
            let deadline = Instant::now() + self.limits.frame_timeout;
            let mut prefix = [0u8; 4];
            if !read_exact(&mut stream, &mut prefix, deadline, true)? {
                return Ok(());
            }
            budget.frame(&self.limits)?;
            let length = u32::from_be_bytes(prefix) as usize;
            if length == 0 || length > self.limits.frame_bytes {
                return Err(Error::Limit("request frame length"));
            }
            budget.charge(length + 4)?;
            let mut payload = Vec::new();
            payload
                .try_reserve_exact(length)
                .map_err(|_| Error::Limit("frame memory"))?;
            payload.resize(length, 0);
            read_exact(&mut stream, &mut payload, deadline, false)?;
            let request = match Request::decode(&payload) {
                Ok(r) => r,
                Err(e) => {
                    let bytes = wire::response(0, Err(&e))?;
                    write_frame(&mut stream, &bytes, &mut budget, &self.limits)?;
                    return Err(e);
                }
            };
            if let Message::Subscribe {
                from_seq,
                max_events,
            } = request.message
            {
                self.subscribe(
                    &mut stream,
                    writer,
                    handlers,
                    staging,
                    &peer,
                    request.id,
                    from_seq,
                    max_events,
                    &mut budget,
                )?;
                continue;
            }
            let result = {
                let mut ledger = writer
                    .lock()
                    .map_err(|_| Error::Invalid("ledger lock poisoned"))?;
                match request.message {
                    Message::Propose(p) => handlers.propose(&mut ledger, &peer, p),
                    Message::Certify {
                        obligation,
                        format,
                        certificate,
                    } => staging
                        .import(&certificate, ledger.cas(), &mut budget.imports)
                        .and_then(|h| handlers.certify(&mut ledger, &peer, obligation, format, h)),
                    Message::Refute { obligation, model } => staging
                        .import(&model, ledger.cas(), &mut budget.imports)
                        .and_then(|h| handlers.refute(&mut ledger, &peer, obligation, h)),
                    Message::Witness { obligation, model } => staging
                        .import(&model, ledger.cas(), &mut budget.imports)
                        .and_then(|h| handlers.witness(&mut ledger, &peer, obligation, h)),
                    Message::Assume {
                        vocabulary,
                        formula,
                        scope,
                    } => handlers.assume(&mut ledger, &peer, vocabulary, formula, scope),
                    Message::Query(query) => handlers.query(&mut ledger, &peer, query),
                    Message::Subscribe { .. } => Err(Error::Invalid("subscription dispatch")),
                }
            };
            let response = match result {
                Ok(value) => wire::response(request.id, Ok(value)),
                Err(error) => wire::response(request.id, Err(&error)),
            }?;
            write_frame(&mut stream, &response, &mut budget, &self.limits)?;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn subscribe<S: Signer, H: Handlers<S>>(
        &self,
        stream: &mut UnixStream,
        writer: &Mutex<&mut Ledger<'_, S>>,
        handlers: &H,
        _staging: &Staging,
        peer: &Peer,
        id: u64,
        mut cursor: u64,
        max: u64,
        budget: &mut Budget,
    ) -> Result<(), Error> {
        let deadline = Instant::now() + self.limits.subscription_timeout;
        let mut sent = 0;
        while sent < max && Instant::now() < deadline {
            let events = {
                let mut ledger = writer
                    .lock()
                    .map_err(|_| Error::Invalid("ledger lock poisoned"))?;
                if ledger.head()?.is_some_and(|h| h.seq >= cursor) {
                    // Limita trabalho síncrono e memória antes de reavaliar o
                    // prazo; não materializa até256 eventos grandes de uma vez.
                    handlers.subscribe(&mut ledger, peer, cursor, 1)?
                } else {
                    Vec::new()
                }
            };
            if events.is_empty() {
                std::thread::sleep(Duration::from_millis(10));
                continue;
            }
            for event in events {
                if Instant::now() >= deadline {
                    break;
                }
                if event.seq != cursor {
                    return Err(Error::Invalid("handler event cursor"));
                }
                let payload = array(vec![Value::Unsigned(0), event.to_value()?])?;
                write_frame_until(
                    stream,
                    &wire::response(id, Ok(payload))?,
                    budget,
                    deadline.min(Instant::now() + self.limits.frame_timeout),
                )?;
                cursor = cursor.checked_add(1).ok_or(Error::Limit("event cursor"))?;
                sent += 1;
            }
        }
        let payload = wire::response(id, Ok(array(vec![Value::Unsigned(1), cursor.to_value()?])?))?;
        if Instant::now() < deadline {
            write_frame_until(
                stream,
                &payload,
                budget,
                deadline.min(Instant::now() + self.limits.frame_timeout),
            )
        } else {
            // O cursor final não renova o prazo de uma assinatura expirada.
            // Se o socket estiver cheio, fecha sem reconhecer evento parcial.
            stream.set_nonblocking(true)?;
            let result = write_frame_io(stream, &payload, budget, None);
            let reset = stream.set_nonblocking(false);
            result?;
            reset?;
            Ok(())
        }
    }
}

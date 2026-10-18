//! Estados fantasma de Claim; não são credenciais construíveis a partir de enums.

/// Proposta registrada, ainda sem obrigação.
#[derive(Debug)]
pub enum Draft {}

/// Obrigação criada e aguardando evidência.
#[derive(Debug)]
pub enum Pending {}

/// Prova aceita e evento Verified registrado.
#[derive(Debug)]
pub enum Verified {}

/// Contramodelo aceito e evento Refuted registrado.
#[derive(Debug)]
pub enum Refuted {}

/// Checagem inconclusiva registrada; terminal nesta instância.
#[derive(Debug)]
pub enum Unknown {}

/// Claim anteriormente verificado, invalidado no ledger.
#[derive(Debug)]
pub enum Invalidated {}

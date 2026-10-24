"""SDK de atestador em Python (6.3): observar, assinar fact/v1 e entregar ao operador.

O atestador é um processo com chave própria; o agente/LLM (``logos_agent``) nunca importa
este pacote nem lê a chave. Assinar não torna o fato citável: o kernel só registra se a
configuração de confiança do ledger conceder ``Trusted(emissor, tópico)`` (6.2). A entrega
é um par de arquivos canônicos (fato + vocabulário) que o operador registra com
``logos ledger fact-register`` (kernel parado) ou que um cliente de atestador entrega ao
kernel. Nada aqui é confiável: o kernel reconfere assinatura, tipagem e janela.
"""

from logos_attest.sdk import (
    Attestation,
    Attester,
    AttestError,
    Ed25519Signer,
    Observation,
    attest,
)

__all__ = [
    "AttestError",
    "Attestation",
    "Attester",
    "Ed25519Signer",
    "Observation",
    "attest",
]

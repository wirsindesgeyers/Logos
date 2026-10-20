# ADR0044 — farm de pesquisa isolada e cache de candidatos

Estado: aceito para implementação da10.2,2026-10-20.

O contém h(V), mas os produtores existentes precisam de V. Receber V público
explicitamente e conferir O/h(O)/h(V) antes do lançamento. Nenhum lookup no
ledger/CAS privado e nenhuma chave é disponibilizada à farm.

Reutilizar prover_bridge/ADR0032 como encoder de apoio fora da autoridade do
kernel. A expressão “grounder não confiável espelhado” do roadmap significa
esta execução sem autoridade: não há uma implementação Python independente.
O kernel rederiva CNF e recheca certificados. Não se altera a TCB.

Executar Z3-refute, finder-refute, SAT e Z3-witness em processos Linux bwrap
com filesystem mínimo, rede/PID/IPC/user namespaces privados, runtime somente
leitura, tmpfs temporário e prlimit. Não montar home, ledger, socket, staging
ou credenciais nos workers. Supervisor recebe socket/staging via cliente10.1.
Primeiro candidato aceito pelo checker ganha; erro, ausência limitada, Unknown
ou candidato rejeitado não interrompem as demais rotas. A busca de testemunha
é independente e sempre iniciada, inclusive com cache; se o estado já encerrou,
registrar a recusa do kernel explicitamente. Não repetir RPC ambíguo.

Cache LRU limitado em memória por h(O), contendo somente bytes de candidatos.
Conferir V em cada chamada; nunca cachear aceitação, ausência ou credencial.
Todo hit passa novamente pelo kernel, inclusive após repróposta/época diferente.
Prazo global monotônico limita produtores e chamadas; cancelamento recolhe
subprocessos. Trabalho síncrono limitado de validação/upload não é preemptível.
Não há promoção de Unknown/Rejected em resultado lógico.

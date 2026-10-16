# Vetores cake-lpr-output/v1 e probe de isolamento

output.json contém14 casos manuais independentes do decoder: stdout/stderr
completos/status/outcome. Aceite exige igualdade exata da saída e exit0;
casos negativos incluem mensagens ambíguas, prefixos, CRLF e aviso em stderr.
A ausência de cláusula vazia e Checking failed usam exit0, observado no
binário real fixado. Fuzz cake_output é semeado com todos os casos.

probe.c e probe-{arch}.xz são somente fixtures de testes do sandbox: verificam
limites, filesystem/environment/network e modo de timeout com descendente/SIGSEGV.
Não passam pelo registry de confiança e não são checkers de prova.
probe.json fixa fonte/hashes. Compilação: Zig0.15.1 cc -target ARCH-linux-musl
-mcpu=baseline -O2 -g0 -s -static -Wl,--build-id=none probe.c -o probe-ARCH.
Os testes comprovam também falha fora do isolamento, conectividade do host,
observação do processo antes do timeout e ausência de descendente após ele.

As200 provas/600 corrupções reais ficam no corpus4.4 em ../lrat/; o teste
corpus executa ambos os checkers, sem usar Rust como filtro do cake_lpr.

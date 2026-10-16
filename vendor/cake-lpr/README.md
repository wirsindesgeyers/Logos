# cake_lpr fixado — ADR0031

Upstream a36874a8b750b43fe4b385b8ddbf5b033e46a3fa; source.tar.xz contém
assembly x86_64/ARMv8, FFI, licença e documentação originais, sem mudanças.
lock.json fixa arquivos/fonte/compilador e hashes SHA-256 crus dos binários.
x86_64.xz/aarch64.xz contêm ELF Linux-musl estáticos para testes offline.

Instalar: `python3 tools/build_cake_lpr.py`.
Reconstruir: `python3 tools/build_cake_lpr.py --rebuild --target x86_64`.
O rebuild baixa Zig0.15.1 por hash, compila em diretório limpo e compara o ELF
com a identidade esperada. O relatório .build.json identifica host/alvo/hash.
A CI prepara dois hosts para o mesmo alvo; a evidência remota não foi observada.
A simples instalação offline não é rebuild.

Verificação CakeML cobre o assembly do checker conforme revisão upstream;
C/FFI, musl, linker, compilador C, kernel e isolamento não têm essa prova.
Os totais Rust da TCB não medem esses componentes nativos. Manifesto registra
cada binário por arquitetura. Fontes preservadas permitem medição independente.

Licença MIT upstream em LICENSE e dentro do arquivo original.

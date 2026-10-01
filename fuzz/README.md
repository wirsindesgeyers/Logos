# Fuzz

Alvos de `cargo fuzz` (exigem toolchain nightly). Cada parser/decoder de TCB ganha um alvo aqui
(roadmap §4): ≥ 1h sem crash no job noturno, com o log arquivado como artefato.

    cargo install cargo-fuzz --locked
    cargo +nightly fuzz run smoke -- -max_total_time=60

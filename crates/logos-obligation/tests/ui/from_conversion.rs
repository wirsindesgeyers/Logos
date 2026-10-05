use logos_obligation::Obligation;

fn main() {
    let bytes = vec![0u8; 32];
    let _o = <Obligation as From<Vec<u8>>>::from(bytes);
}

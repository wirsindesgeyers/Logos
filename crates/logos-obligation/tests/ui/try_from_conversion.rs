use logos_obligation::Obligation;

fn main() {
    let bytes = [0u8; 32];
    let _res: Result<Obligation, _> = Obligation::try_from(&bytes[..]);
}

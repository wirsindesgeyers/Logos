use logos_canon::from_canon;
use logos_obligation::Obligation;

fn main() {
    let bytes = [0u8; 32];
    let _res: Result<Obligation, _> = from_canon(&bytes);
}

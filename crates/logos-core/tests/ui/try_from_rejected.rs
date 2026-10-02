use logos_canon::Text;
use logos_core::{CheckOutcome, Reason, Verdict};

fn main() {
    let text = Text::new("falha").unwrap();
    let outcome = CheckOutcome::Rejected(Reason::new(text));
    let _verdict: Result<Verdict, _> = Verdict::try_from(outcome);
}

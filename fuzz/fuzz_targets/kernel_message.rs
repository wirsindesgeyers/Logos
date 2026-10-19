#![no_main]
use libfuzzer_sys::fuzz_target;
use logos_kernel::wire::{Request, decode_frame, encode_frame};
fuzz_target!(|data: &[u8]| {
    if let Ok(request) = Request::decode(data) {
        assert_eq!(request.encode().as_deref().ok(), Some(data));
        let frame = encode_frame(&request).expect("accepted request encodes");
        assert_eq!(decode_frame(&frame).ok(), Some(request));
    }
    if let Ok(request) = decode_frame(data) {
        assert_eq!(encode_frame(&request).as_deref().ok(), Some(data));
    }
});

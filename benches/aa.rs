#[macro_use]
extern crate bencher;

use bencher::Bencher;
use bytes::BytesMut;

use bravery::http_protocol::HttpProtocolParser;

fn a(bench: &mut Bencher) {
    bench.iter(|| {
        let mut input = BytesMut::new();
        input.extend_from_slice(b"POST / HTTP/1.1\r\nHost:localhost:8880\r\nContent-Length: 0\r\nAccept: */*\r\n\r\n");

        let mut headers = [httparse::EMPTY_HEADER; 16];
        let mut req = httparse::Request::new(&mut headers);
        req.parse(&input).unwrap();

        input.split_to(71);
    })
}

fn b(bench: &mut Bencher) {
    bench.iter(|| {
        let mut input = BytesMut::new();
        input.extend_from_slice(b"POST / HTTP/1.1\r\nHost:localhost:8880\r\nContent-Length: 0\r\nAccept: */*\r\n\r\n");

        let parser = HttpProtocolParser {};

        parser.parse(&mut input);
    });
}

benchmark_group!(benches, a, b);
benchmark_main!(benches);

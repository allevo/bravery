#[macro_use]
extern crate bencher;

use bencher::Bencher;
use bytes::BytesMut;

fn b(bench: &mut Bencher) {
    bench.iter(|| {
        let mut input = BytesMut::new();
        input.extend_from_slice(b"Host:localhost:8880\r\nContent-Length: 0\r\nAccept: */*\r\n\r\n");

        let mut content_length = None;
        let mut _content_type = None;
        for i in 0..input.len() {
            if true &&
                input[i] == b'C' &&
                input[i + 1] == b'o' &&
                input[i + 2] == b'n' &&
                input[i + 3] == b't' &&
                input[i + 4] == b'e' &&
                input[i + 5] == b'n' &&
                input[i + 6] == b't' &&
                input[i + 7] == b'-' &&
                input[i + 8] == b'L' &&
                input[i + 9] == b'e' &&
                input[i + 10] == b'n' &&
                input[i + 11] == b'g' &&
                input[i + 12] == b't' &&
                input[i + 13] == b'h' &&
                input[i + 14] == b':'
            {
                for j in i..input.len() {
                    if input[j] == b'\r' {
                        content_length = Some(&input[i..j]);
                        break;
                    }
                }
                break;
            }

            if true &&
                input[i] == b'C' &&
                input[i + 1] == b'o' &&
                input[i + 2] == b'n' &&
                input[i + 3] == b't' &&
                input[i + 4] == b'e' &&
                input[i + 5] == b'n' &&
                input[i + 6] == b't' &&
                input[i + 7] == b'-' &&
                input[i + 8] == b'T' &&
                input[i + 9] == b'y' &&
                input[i + 10] == b'p' &&
                input[i + 11] == b'e' &&
                input[i + 12] == b':'
            {
                for j in i..input.len() {
                    if input[j] == b'\r' {
                        _content_type = Some(&input[i..j]);
                        break;
                    }
                }
                break;
            }
        }

        // content_type.unwrap();
        content_length.unwrap();
    });
}

fn c(bench: &mut Bencher) {
    bench.iter(|| {
        let mut input = BytesMut::new();
        input.extend_from_slice(b"Host:localhost:8880\r\nContent-Length: 0\r\nAccept: */*\r\n\r\n");

        let mut content_length = None;
        let mut _content_type = None;
        for i in 0..input.len() {
            if true &&
                input[i] == b'C' &&
                input[i + 1] == b'o' &&
                input[i + 2] == b'n' &&
                input[i + 3] == b't' &&
                input[i + 4] == b'e' &&
                input[i + 5] == b'n' &&
                input[i + 6] == b't' &&
                input[i + 7] == b'-' &&
                input[i + 8] == b'L' &&
                input[i + 9] == b'e' &&
                input[i + 10] == b'n' &&
                input[i + 11] == b'g' &&
                input[i + 12] == b't' &&
                input[i + 13] == b'h' &&
                input[i + 14] == b':'
            {
                for j in input.iter().enumerate().skip(i) {
                    if *j.1 == b'\r' {
                        content_length = Some((i, j.0));
                        break;
                    }
                }
                break;
            }

            if true &&
                input[i] == b'C' &&
                input[i + 1] == b'o' &&
                input[i + 2] == b'n' &&
                input[i + 3] == b't' &&
                input[i + 4] == b'e' &&
                input[i + 5] == b'n' &&
                input[i + 6] == b't' &&
                input[i + 7] == b'-' &&
                input[i + 8] == b'T' &&
                input[i + 9] == b'y' &&
                input[i + 10] == b'p' &&
                input[i + 11] == b'e' &&
                input[i + 12] == b':'
            {
                for j in input.iter().enumerate().skip(i) {
                    if *j.1 == b'\r' {
                        _content_type = Some((i, j.0));
                        break;
                    }
                }
                break;
            }
        }

        // content_type.unwrap();
        content_length.unwrap();
    });
}

benchmark_group!(benches, b, c);
benchmark_main!(benches);

use lambda_sdk::{
    debug::debug_str,
    entry,
    interops::{SlicePointer, StringPointer},
    io::{RequestData, Response},
    net::{send_request, Request},
    vault::Secret,
};

entry!(main);

fn main() {
    debug_str("Hello, WASM!");

    let mut request_data: RequestData = RequestData::new().expect("No data!");

    debug_str("Hello, WASM!");

    let mut buf: Vec<u8> = vec![0; request_data.len().max(u16::MAX.into()).try_into().unwrap()];

    debug_str(&format!("{:?}", buf.len()));

    let count: usize = request_data.read(&mut buf).len();

    debug_str(&format!("{:?}", &buf[..count]));

    Response::write(&buf[..count]);

    let mut response = send_request(&Request {
        method: StringPointer::from("GET"),
        url: StringPointer::from("https://dir.bg"),
        headers: SlicePointer::from(&[]),
        body: SlicePointer::from(&[]),
    })
    .unwrap();

    debug_str(&format!("Status code: {}", response.status_code()));

    let mut buf: Vec<u8> = vec![0; 64];

    Response::write(response.read(&mut buf));

    debug_str(&format!("Status code: {}", response.status_code()));

    let secret = Secret::fetch_secret("123");

    if let Err(secret) = secret {
        debug_str(&format!("{:?}", secret));

        panic!("{:?}", secret);
    }

    let mut secret = secret.unwrap();

    debug_str(&format!("{:?}", secret.read(&mut buf)));
}

//! 响应体解码 — infrastructure/parsers/
//!
//! 上游可能对响应做**传输层压缩**。例如 DeepSeek 边缘节点对 SSE 返回
//! `content-encoding: zstd`（同时 `content-type: text/event-stream`），此时
//! 累积到的字节是压缩流：直接当文本落库会让词元解析与详情页渲染全部失效
//! （表现为「所有词元为 0」「轨迹 0 消息 0 工具」）。
//!
//! 因此**在落库与解析之前**按 `content-encoding` 解码；解码只作用于日志副本，
//! 转发给客户端的仍是上游原始字节（保持字节保真透传）。
//!
//! 支持的编码：`zstd` / `gzip` / `x-gzip` / `deflate` / `br` / `identity`，
//! 以及逗号分隔的多重编码（按声明逆序解码）。任何失败都**回退原字节**，
//! 绝不因解码问题丢日志或打断转发。

use std::io::Read;

/// 按 `content-encoding` 解码响应体；未知编码或解码失败时返回原字节
pub fn decode_body(bytes: &[u8], content_encoding: Option<&str>) -> Vec<u8> {
    let Some(encoding) = content_encoding else {
        return bytes.to_vec();
    };

    // 多重编码形如 "gzip, br"：最外层在后，故逆序解
    let mut data = bytes.to_vec();
    for enc in encoding
        .split(',')
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .rev()
    {
        match enc.as_str() {
            "identity" => {}
            "zstd" => match zstd_decode(&data) {
                Some(out) => data = out,
                None => return bytes.to_vec(),
            },
            "gzip" | "x-gzip" => match gzip_decode(&data) {
                Some(out) => data = out,
                None => return bytes.to_vec(),
            },
            "deflate" => match deflate_decode(&data) {
                Some(out) => data = out,
                None => return bytes.to_vec(),
            },
            "br" => match brotli_decode(&data) {
                Some(out) => data = out,
                None => return bytes.to_vec(),
            },
            // 未知编码（如客户端自定义编码）：不猜测，原样保留
            _ => return bytes.to_vec(),
        }
    }
    data
}

fn read_all<R: Read>(mut reader: R) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    reader.read_to_end(&mut out).ok().map(|_| out)
}

fn zstd_decode(data: &[u8]) -> Option<Vec<u8>> {
    zstd::stream::decode_all(data).ok()
}

fn gzip_decode(data: &[u8]) -> Option<Vec<u8>> {
    read_all(flate2::read::GzDecoder::new(data))
}

/// HTTP 的 `deflate` 规范上是 zlib 包装；部分实现直接发裸 deflate，故兜底再试一次
fn deflate_decode(data: &[u8]) -> Option<Vec<u8>> {
    read_all(flate2::read::ZlibDecoder::new(data))
        .or_else(|| read_all(flate2::read::DeflateDecoder::new(data)))
}

fn brotli_decode(data: &[u8]) -> Option<Vec<u8>> {
    read_all(brotli::Decompressor::new(data, 4096))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const PLAIN: &[u8] = b"event: message_start\ndata: {\"type\":\"message_start\"}\n\n";

    fn zstd(data: &[u8]) -> Vec<u8> {
        zstd::stream::encode_all(data, 3).unwrap()
    }

    fn gzip(data: &[u8]) -> Vec<u8> {
        let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        enc.write_all(data).unwrap();
        enc.finish().unwrap()
    }

    fn zlib(data: &[u8]) -> Vec<u8> {
        let mut enc = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        enc.write_all(data).unwrap();
        enc.finish().unwrap()
    }

    fn brotli(data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut w = brotli::CompressorWriter::new(&mut out, 4096, 5, 22);
            w.write_all(data).unwrap();
        }
        out
    }

    #[test]
    fn test_zstd_sse_stream_is_decoded() {
        // 本次生产故障的复现形态：zstd 压缩的 SSE
        let decoded = decode_body(&zstd(PLAIN), Some("zstd"));
        assert_eq!(decoded, PLAIN);
        assert!(String::from_utf8_lossy(&decoded).contains("message_start"));
    }

    #[test]
    fn test_gzip_deflate_brotli_decoded() {
        assert_eq!(decode_body(&gzip(PLAIN), Some("gzip")), PLAIN);
        assert_eq!(decode_body(&gzip(PLAIN), Some("x-gzip")), PLAIN);
        assert_eq!(decode_body(&zlib(PLAIN), Some("deflate")), PLAIN);
        assert_eq!(decode_body(&brotli(PLAIN), Some("br")), PLAIN);
    }

    #[test]
    fn test_case_insensitive_and_whitespace() {
        assert_eq!(decode_body(&zstd(PLAIN), Some(" ZSTD ")), PLAIN);
    }

    #[test]
    fn test_multi_encoding_applied_in_reverse_order() {
        // HTTP 语义：`Content-Encoding: a, b` 表示先按 a 编码、再按 b 编码（b 在最外层），
        // 因此解码时从后往前（先 b 后 a）。
        // 载荷 = zstd(gzip(明文))：最外层是 zstd，声明 "gzip, zstd"
        assert_eq!(decode_body(&zstd(&gzip(PLAIN)), Some("gzip, zstd")), PLAIN);
        // 载荷 = gzip(zstd(明文))：最外层是 gzip，声明 "zstd, gzip"
        assert_eq!(decode_body(&gzip(&zstd(PLAIN)), Some("zstd, gzip")), PLAIN);
    }

    #[test]
    fn test_identity_and_missing_encoding_keep_bytes() {
        assert_eq!(decode_body(PLAIN, None), PLAIN);
        assert_eq!(decode_body(PLAIN, Some("identity")), PLAIN);
    }

    #[test]
    fn test_unknown_encoding_keeps_bytes() {
        assert_eq!(decode_body(PLAIN, Some("snappy")), PLAIN);
    }

    #[test]
    fn test_corrupted_payload_falls_back_to_original() {
        let garbage = b"\xff\xfe\xfd not really zstd";
        assert_eq!(decode_body(garbage, Some("zstd")), garbage);
    }

    #[test]
    fn test_empty_body() {
        assert_eq!(decode_body(b"", Some("zstd")), b"");
    }
}

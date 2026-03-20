//! Lark long-connection binary frame codec (protobuf pbbp2).
//!
//! The official Lark Go SDK uses protobuf-encoded binary WebSocket frames.
//! This module defines the `Frame` and `Header` structs using `prost` derive
//! macros (no protoc required).

/// Protobuf frame header (key-value pair).
#[derive(Clone, PartialEq, prost::Message)]
pub struct Header {
    #[prost(string, required, tag = "1")]
    pub key: String,
    #[prost(string, required, tag = "2")]
    pub value: String,
}

/// Lark long-connection binary frame (pbbp2 protocol).
#[derive(Clone, PartialEq, prost::Message)]
pub struct Frame {
    #[prost(uint64, required, tag = "1")]
    pub seq_id: u64,
    #[prost(uint64, required, tag = "2")]
    pub log_id: u64,
    #[prost(int32, required, tag = "3")]
    pub service: i32,
    #[prost(int32, required, tag = "4")]
    pub method: i32,
    #[prost(message, repeated, tag = "5")]
    pub headers: Vec<Header>,
    #[prost(string, optional, tag = "6")]
    pub payload_encoding: Option<String>,
    #[prost(string, optional, tag = "7")]
    pub payload_type: Option<String>,
    #[prost(bytes = "vec", optional, tag = "8")]
    pub payload: Option<Vec<u8>>,
    #[prost(string, optional, tag = "9")]
    pub log_id_new: Option<String>,
}

/// Frame method: control (ping/pong).
pub const FRAME_CONTROL: i32 = 0;
/// Frame method: data (event/card payload).
pub const FRAME_DATA: i32 = 1;

// Header key constants (matching Go SDK const.go).
pub const HEADER_TYPE: &str = "type";
pub const HEADER_MESSAGE_ID: &str = "message_id";
pub const HEADER_SUM: &str = "sum";
pub const HEADER_SEQ: &str = "seq";
pub const HEADER_TRACE_ID: &str = "trace_id";
pub const HEADER_INSTANCE_ID: &str = "instance_id";
pub const HEADER_BIZ_RT: &str = "biz_rt";
pub const HEADER_TIMESTAMP: &str = "timestamp";

impl Header {
    pub fn new(key: &str, value: &str) -> Self {
        Self {
            key: key.to_string(),
            value: value.to_string(),
        }
    }
}

impl Frame {
    /// Look up a header value by key.
    pub fn get_header(&self, key: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|h| h.key == key)
            .map(|h| h.value.as_str())
    }

    /// Create a ping control frame.
    pub fn ping(service_id: i32) -> Self {
        Self {
            seq_id: 0,
            log_id: 0,
            service: service_id,
            method: FRAME_CONTROL,
            headers: vec![Header {
                key: HEADER_TYPE.to_string(),
                value: "ping".to_string(),
            }],
            payload_encoding: None,
            payload_type: None,
            payload: None,
            log_id_new: None,
        }
    }

    /// Create a response/ack frame by reusing the incoming frame's identity
    /// fields (seq_id, log_id, service, headers) — matching the official Go SDK
    /// behaviour where the original frame is modified in-place before sending back.
    ///
    /// `status_code` uses HTTP status codes (200 = OK, 500 = error) to match
    /// the Go SDK `Response` struct: `{"code":200,"headers":null,"data":null}`.
    ///
    /// `biz_rt_ms` is the business processing time in milliseconds; it is
    /// appended as the `biz_rt` header so the server can track handler latency.
    pub fn into_response(mut self, status_code: i32, biz_rt_ms: i64) -> Self {
        // Match Go SDK Response struct: {"code":200,"headers":null,"data":null}
        let payload = serde_json::json!({ "code": status_code, "headers": null, "data": null });
        // Append biz_rt header (processing duration in ms)
        self.headers
            .push(Header::new(HEADER_BIZ_RT, &biz_rt_ms.to_string()));
        self.payload = Some(payload.to_string().into_bytes());
        self
    }

    /// Create a response/ack frame for data messages with custom headers.
    ///
    /// **Prefer [`into_response`] for data frame ACKs** — it preserves the
    /// original frame's seq_id/log_id which Lark requires for matching.
    /// This method is kept for cases where a standalone frame is needed.
    pub fn response_with_headers(service: i32, status_code: i32, headers: Vec<Header>) -> Self {
        let payload = serde_json::json!({ "code": status_code, "headers": null, "data": null });
        Self {
            seq_id: 0,
            log_id: 0,
            service,
            method: FRAME_DATA,
            headers,
            payload_encoding: None,
            payload_type: None,
            payload: Some(payload.to_string().into_bytes()),
            log_id_new: None,
        }
    }

    /// Create a response/ack frame for data messages.
    ///
    /// **Prefer [`into_response`] for data frame ACKs** — it preserves the
    /// original frame's seq_id/log_id which Lark requires for matching.
    pub fn response(service: i32, status_code: i32) -> Self {
        let payload = serde_json::json!({ "code": status_code, "headers": null, "data": null });
        Self {
            seq_id: 0,
            log_id: 0,
            service,
            method: FRAME_DATA,
            headers: vec![],
            payload_encoding: None,
            payload_type: None,
            payload: Some(payload.to_string().into_bytes()),
            log_id_new: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prost::Message;

    #[test]
    fn roundtrip_ping_frame() {
        let frame = Frame::ping(7);
        let bytes = frame.encode_to_vec();
        let decoded = Frame::decode(bytes.as_slice()).unwrap();
        assert_eq!(decoded.method, FRAME_CONTROL);
        assert_eq!(decoded.service, 7);
        assert_eq!(decoded.get_header(HEADER_TYPE), Some("ping"));
    }

    #[test]
    fn roundtrip_response_frame() {
        let frame = Frame::response(7, 200);
        let bytes = frame.encode_to_vec();
        let decoded = Frame::decode(bytes.as_slice()).unwrap();
        assert_eq!(decoded.method, FRAME_DATA);
        let payload_str = String::from_utf8(decoded.payload.unwrap()).unwrap();
        let v: serde_json::Value = serde_json::from_str(&payload_str).unwrap();
        assert_eq!(v["code"], 200);
        assert!(v["headers"].is_null());
        assert!(v["data"].is_null());
    }

    #[test]
    fn roundtrip_response_with_headers_frame() {
        let headers = vec![
            Header::new(HEADER_TRACE_ID, "abc-123"),
            Header::new(HEADER_INSTANCE_ID, "inst-1"),
        ];
        let frame = Frame::response_with_headers(7, 200, headers);
        let bytes = frame.encode_to_vec();
        let decoded = Frame::decode(bytes.as_slice()).unwrap();
        assert_eq!(decoded.method, FRAME_DATA);
        assert_eq!(decoded.service, 7);
        assert_eq!(decoded.get_header(HEADER_TRACE_ID), Some("abc-123"));
        assert_eq!(decoded.get_header(HEADER_INSTANCE_ID), Some("inst-1"));
        let payload_str = String::from_utf8(decoded.payload.unwrap()).unwrap();
        let v: serde_json::Value = serde_json::from_str(&payload_str).unwrap();
        assert_eq!(v["code"], 200);
        assert!(v["headers"].is_null());
        assert!(v["data"].is_null());
    }

    #[test]
    fn into_response_preserves_identity() {
        let frame = Frame {
            seq_id: 42,
            log_id: 123,
            service: 7,
            method: FRAME_DATA,
            headers: vec![Header::new(HEADER_TRACE_ID, "t-1")],
            payload_encoding: None,
            payload_type: None,
            payload: Some(b"original".to_vec()),
            log_id_new: None,
        };
        let resp = frame.into_response(200, 15);
        assert_eq!(resp.seq_id, 42, "seq_id must be preserved");
        assert_eq!(resp.log_id, 123, "log_id must be preserved");
        assert_eq!(resp.service, 7);
        assert_eq!(resp.get_header(HEADER_TRACE_ID), Some("t-1"));
        assert_eq!(resp.get_header(HEADER_BIZ_RT), Some("15"));
        let payload_str = String::from_utf8(resp.payload.unwrap()).unwrap();
        let v: serde_json::Value = serde_json::from_str(&payload_str).unwrap();
        assert_eq!(v["code"], 200);
        assert!(v["headers"].is_null());
        assert!(v["data"].is_null());
    }

    #[test]
    fn get_header_missing() {
        let frame = Frame::ping(1);
        assert_eq!(frame.get_header("nonexistent"), None);
    }
}

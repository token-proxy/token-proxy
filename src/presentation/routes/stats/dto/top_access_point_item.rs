use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct TopAccessPointItem {
    /// 接入点 ID
    pub access_point_id: Uuid,
    /// 请求次数
    pub count: u64,
}

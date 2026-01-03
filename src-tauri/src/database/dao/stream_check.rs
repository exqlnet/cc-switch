//! 流式健康检查日志 DAO

use crate::database::{lock_conn, Database};
use crate::error::AppError;
use crate::services::stream_check::{HealthStatus, StreamCheckConfig, StreamCheckResult};

impl Database {
    /// 保存流式检查日志
    pub fn save_stream_check_log(
        &self,
        provider_id: &str,
        provider_name: &str,
        app_type: &str,
        result: &StreamCheckResult,
    ) -> Result<i64, AppError> {
        let conn = lock_conn!(self.conn);

        conn.execute(
            "INSERT INTO stream_check_logs 
             (provider_id, provider_name, app_type, status, success, message, 
              response_time_ms, http_status, model_used, retry_count, tested_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                provider_id,
                provider_name,
                app_type,
                format!("{:?}", result.status).to_lowercase(),
                result.success,
                result.message,
                result.response_time_ms.map(|t| t as i64),
                result.http_status.map(|s| s as i64),
                result.model_used,
                result.retry_count as i64,
                result.tested_at,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(conn.last_insert_rowid())
    }

    /// 获取流式检查配置
    pub fn get_stream_check_config(&self) -> Result<StreamCheckConfig, AppError> {
        match self.get_setting("stream_check_config")? {
            Some(json) => serde_json::from_str(&json)
                .map_err(|e| AppError::Message(format!("解析配置失败: {e}"))),
            None => Ok(StreamCheckConfig::default()),
        }
    }

    /// 保存流式检查配置
    pub fn save_stream_check_config(&self, config: &StreamCheckConfig) -> Result<(), AppError> {
        let json = serde_json::to_string(config)
            .map_err(|e| AppError::Message(format!("序列化配置失败: {e}")))?;
        self.set_setting("stream_check_config", &json)
    }

    /// 获取某个 Provider 最近一次流式检查结果（来自日志）
    pub fn get_stream_check_latest(
        &self,
        provider_id: &str,
        app_type: &str,
    ) -> Result<Option<StreamCheckResult>, AppError> {
        let conn = lock_conn!(self.conn);

        let row = conn.query_row(
            "SELECT status, success, message, response_time_ms, http_status, model_used, retry_count, tested_at
             FROM stream_check_logs
             WHERE provider_id = ?1 AND app_type = ?2
             ORDER BY tested_at DESC, id DESC
             LIMIT 1",
            rusqlite::params![provider_id, app_type],
            |row| {
                let status_str: String = row.get(0)?;
                let success: bool = row.get(1)?;
                let message: String = row.get(2)?;
                let response_time_ms: Option<i64> = row.get(3)?;
                let http_status: Option<i64> = row.get(4)?;
                let model_used: Option<String> = row.get(5)?;
                let retry_count: Option<i64> = row.get(6)?;
                let tested_at: i64 = row.get(7)?;

                let status = match status_str.as_str() {
                    "operational" => HealthStatus::Operational,
                    "degraded" => HealthStatus::Degraded,
                    _ => HealthStatus::Failed,
                };

                Ok(StreamCheckResult {
                    status,
                    success,
                    message,
                    response_time_ms: response_time_ms.map(|v| v as u64),
                    http_status: http_status.map(|v| v as u16),
                    model_used: model_used.unwrap_or_default(),
                    tested_at,
                    retry_count: retry_count.unwrap_or(0) as u32,
                })
            },
        );

        match row {
            Ok(r) => Ok(Some(r)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    /// 获取某个 Provider 最近 N 次流式检查结果（来自日志，按时间倒序）
    pub fn get_stream_check_history(
        &self,
        provider_id: &str,
        app_type: &str,
        limit: u32,
    ) -> Result<Vec<StreamCheckResult>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare(
                "SELECT status, success, message, response_time_ms, http_status, model_used, retry_count, tested_at
                 FROM stream_check_logs
                 WHERE provider_id = ?1 AND app_type = ?2
                 ORDER BY tested_at DESC, id DESC
                 LIMIT ?3",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let rows = stmt
            .query_map(
                rusqlite::params![provider_id, app_type, limit as i64],
                |row| {
                    let status_str: String = row.get(0)?;
                    let success: bool = row.get(1)?;
                    let message: String = row.get(2)?;
                    let response_time_ms: Option<i64> = row.get(3)?;
                    let http_status: Option<i64> = row.get(4)?;
                    let model_used: Option<String> = row.get(5)?;
                    let retry_count: Option<i64> = row.get(6)?;
                    let tested_at: i64 = row.get(7)?;

                    let status = match status_str.as_str() {
                        "operational" => HealthStatus::Operational,
                        "degraded" => HealthStatus::Degraded,
                        _ => HealthStatus::Failed,
                    };

                    Ok(StreamCheckResult {
                        status,
                        success,
                        message,
                        response_time_ms: response_time_ms.map(|v| v as u64),
                        http_status: http_status.map(|v| v as u16),
                        model_used: model_used.unwrap_or_default(),
                        tested_at,
                        retry_count: retry_count.unwrap_or(0) as u32,
                    })
                },
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| AppError::Database(e.to_string()))?);
        }

        Ok(results)
    }
}

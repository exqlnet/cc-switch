//! 代理 TPS 监控（滑动窗口）
//!
//! 目标：在代理模式下对真实用户请求的输出 token 进行滑动窗口聚合，并以 TPS（token/秒）形式暴露。

use std::{collections::VecDeque, time::Duration};

/// 默认统计窗口（秒）
pub const DEFAULT_WINDOW_SECS: u64 = 5;

#[derive(Debug, Clone)]
struct RequestSegment {
    start: std::time::Instant,
    end: std::time::Instant,
    output_tokens: u64,
}

/// TPS 监控器（输出 token/秒）
#[derive(Debug)]
pub struct TpsMonitor {
    window: Duration,
    segments: VecDeque<RequestSegment>,
}

impl TpsMonitor {
    pub fn new(window_secs: u64) -> Self {
        Self {
            window: Duration::from_secs(window_secs.max(1)),
            segments: VecDeque::new(),
        }
    }

    /// 记录一次“请求完成”的输出 token（不做估算）
    ///
    /// 口径：将该请求的输出 token 按请求持续时间（start->end）均匀摊销，
    /// 再在查询时按“最近 window 秒”与请求区间的重叠来计算贡献。
    pub fn record_completed_request(
        &mut self,
        output_tokens: u64,
        start: std::time::Instant,
        end: std::time::Instant,
    ) {
        if output_tokens == 0 {
            return;
        }

        if end <= start {
            return;
        }

        self.segments.push_back(RequestSegment {
            start,
            end,
            output_tokens,
        });
        self.trim_expired_at(end);
    }

    /// 获取当前 TPS（滑动窗口聚合 / 固定窗口秒数）
    ///
    /// - 空闲（窗口内无条目）返回 0
    /// - 采用固定窗口除数（例如 5 秒），避免窗口刚开始时数值异常抖动
    pub fn current_tps(&mut self) -> f64 {
        self.current_tps_at(std::time::Instant::now())
    }

    pub fn current_tps_at(&mut self, now: std::time::Instant) -> f64 {
        self.trim_expired_at(now);

        let window_secs = self.window.as_secs_f64();
        if window_secs <= 0.0 {
            return 0.0;
        }

        let window_start = match now.checked_sub(self.window) {
            Some(t) => t,
            None => return 0.0,
        };

        let mut tokens_in_window = 0.0f64;

        for seg in self.segments.iter() {
            if seg.end <= window_start {
                continue;
            }
            if seg.start >= now {
                continue;
            }

            let overlap_start = std::cmp::max(seg.start, window_start);
            let overlap_end = std::cmp::min(seg.end, now);
            if overlap_end <= overlap_start {
                continue;
            }

            let seg_secs = (seg.end - seg.start).as_secs_f64();
            if seg_secs <= 0.0 {
                continue;
            }

            let overlap_secs = (overlap_end - overlap_start).as_secs_f64();
            if overlap_secs <= 0.0 {
                continue;
            }

            tokens_in_window += (seg.output_tokens as f64) * (overlap_secs / seg_secs);
        }

        if tokens_in_window <= 0.0 {
            return 0.0;
        }

        tokens_in_window / window_secs
    }

    pub fn reset(&mut self) {
        self.segments.clear();
    }

    fn trim_expired_at(&mut self, now: std::time::Instant) {
        let Some(cutoff) = now.checked_sub(self.window) else {
            return;
        };

        while let Some(front) = self.segments.front() {
            // 请求区间整体结束时间早于窗口左边界，则永不再贡献
            if front.end < cutoff {
                self.segments.pop_front();
            } else {
                break;
            }
        }
    }
}

impl Default for TpsMonitor {
    fn default() -> Self {
        Self::new(DEFAULT_WINDOW_SECS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_zero() {
        let mut m = TpsMonitor::new(5);
        assert_eq!(m.current_tps(), 0.0);
    }

    #[test]
    fn ignores_zero_tokens() {
        let mut m = TpsMonitor::new(5);
        let end = std::time::Instant::now();
        let start = end - Duration::from_secs(1);
        m.record_completed_request(0, start, end);
        assert_eq!(m.segments.len(), 0);
        assert_eq!(m.current_tps(), 0.0);
    }

    #[test]
    fn basic_tps() {
        let mut m = TpsMonitor::new(5);
        let end = std::time::Instant::now();
        let start = end - Duration::from_secs(10);
        m.record_completed_request(100, start, end);

        // 最近 5 秒窗口与请求区间重叠 5 秒：token_in_window=100*(5/10)=50，TPS=50/5=10
        assert!((m.current_tps_at(end) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn expires_out_of_window() {
        let mut m = TpsMonitor::new(5);
        let end = std::time::Instant::now();
        let start = end - Duration::from_secs(10);
        m.record_completed_request(100, start, end);

        // 窗口完全移出请求区间后 TPS 归零
        let later = end + Duration::from_secs(6);
        assert_eq!(m.current_tps_at(later), 0.0);
    }
}

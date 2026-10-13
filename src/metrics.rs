//! Lightweight stage timings emitted to the existing rotating container log.
pub struct Stage {
    name: &'static str,
    start: std::time::Instant,
    http_start: (u64, u64),
}
impl Stage {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            start: std::time::Instant::now(),
            http_start: crate::http::text_metrics(),
        }
    }
}
impl Drop for Stage {
    fn drop(&mut self) {
        let (responses, bytes) = crate::http::text_metrics();
        eprintln!(
            "metrics: stage={} elapsed_ms={} http_text_responses={} http_text_bytes={}",
            self.name,
            self.start.elapsed().as_millis(),
            responses.saturating_sub(self.http_start.0),
            bytes.saturating_sub(self.http_start.1)
        );
    }
}

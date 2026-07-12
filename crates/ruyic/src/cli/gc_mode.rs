//! GC 模式枚举与 `--gc=<mode>` 标志解析。
//!
//! Ruyi 编译器支持两种 GC 模式：
//! - `stub`: 占位分配器（默认），编译快，不启用真实 GC
//! - `real`: 真实 generational GC，链入 `ruyi_runtime`，编译慢
//!
//! @author luozegang
//! @date 2026-07-10

use std::fmt;
use std::str::FromStr;

/// GC 模式枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GcMode {
    /// 占位分配器（默认）
    Stub,
    /// 真实 generational GC
    Real,
}

impl GcMode {
    /// 解析 `--gc=<mode>` 字符串。
    ///
    /// 接受 `"stub"` 或 `"real"`，其他返回 Err。
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "stub" => Ok(GcMode::Stub),
            "real" => Ok(GcMode::Real),
            other => Err(format!(
                "invalid GC mode '{}': expected 'stub' or 'real'",
                other
            )),
        }
    }

    /// 返回字面量字符串（用于错误信息）
    pub fn as_str(&self) -> &'static str {
        match self {
            GcMode::Stub => "stub",
            GcMode::Real => "real",
        }
    }
}

impl Default for GcMode {
    fn default() -> Self {
        GcMode::Stub
    }
}

impl fmt::Display for GcMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for GcMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TDD-RED 起点：验证 `parse("stub")` 返回 Ok(Stub)
    #[test]
    fn parse_stub_returns_stub() {
        assert_eq!(GcMode::parse("stub").unwrap(), GcMode::Stub);
    }

    #[test]
    fn parse_real_returns_real() {
        assert_eq!(GcMode::parse("real").unwrap(), GcMode::Real);
    }

    #[test]
    fn parse_invalid_returns_err() {
        assert!(GcMode::parse("invalid").is_err());
        assert!(GcMode::parse("").is_err());
        assert!(GcMode::parse("Stub").is_err(), "大小写敏感：拒绝 'Stub'");
    }

    #[test]
    fn default_is_stub() {
        assert_eq!(GcMode::default(), GcMode::Stub);
    }

    #[test]
    fn as_str_roundtrips() {
        assert_eq!(GcMode::Stub.as_str(), "stub");
        assert_eq!(GcMode::Real.as_str(), "real");
    }

    #[test]
    fn from_str_works() {
        let mode: GcMode = "real".parse().unwrap();
        assert_eq!(mode, GcMode::Real);
    }

    #[test]
    fn display_matches_as_str() {
        assert_eq!(format!("{}", GcMode::Stub), "stub");
        assert_eq!(format!("{}", GcMode::Real), "real");
    }
}

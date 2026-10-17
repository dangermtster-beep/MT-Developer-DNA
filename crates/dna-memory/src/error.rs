use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("Proje kökü mevcut, gerçek bir klasör olmalıdır: {0}")]
    InvalidProjectRoot(PathBuf),
    #[error("Proje dışına çıkan veya güvenli olmayan yol reddedildi: {0}")]
    UnsafePath(String),
    #[error("Symlink/reparse yolu reddedildi: {0}")]
    LinkPath(PathBuf),
    #[error("Proje hafızası zaten var; otomatik sıfırlanmadı: {0}")]
    AlreadyInitialized(PathBuf),
    #[error("Proje hafızası bulunamadı: {0}")]
    NotInitialized(PathBuf),
    #[error("Hafıza yazma kilidi alınamadı; başka bir işlem etkin olabilir: {0}")]
    Locked(PathBuf),
    #[error("Hafıza planlandıktan sonra değişti; yeniden önizleme alın: {0}")]
    Conflict(String),
    #[error("Desteklenmeyen hafıza şeması: {found}. Beklenen: {expected}. Önce `memory repair` önizlemesi alın.")]
    UnsupportedSchema { found: String, expected: String },
    #[error("Geçersiz hafıza alanı `{field}`: {message}")]
    InvalidField {
        field: &'static str,
        message: String,
    },
    #[error("Hafıza JSON dosyası geçersiz ({path}): {source}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("{operation} başarısız ({path}): {source}")]
    Io {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

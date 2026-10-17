use crate::{validate_profile, ProfileLoadError, ValidationErrors};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const PROFILE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub schema_version: u32,
    pub name: String,
    pub language: ProgrammingLanguage,
    pub ui: UiFramework,
    pub database: Database,
    #[serde(default)]
    pub features: FeaturePreferences,
    #[serde(default)]
    pub standards: CommonStandards,
}

impl Profile {
    pub fn from_json_str(json: &str) -> Result<Self, ProfileLoadError> {
        let profile: Self = serde_json::from_str(json)?;
        profile.validate()?;
        Ok(profile)
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, ProfileLoadError> {
        let path = path.as_ref();
        let json = fs::read_to_string(path).map_err(|source| ProfileLoadError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        Self::from_json_str(&json)
    }

    pub fn validate(&self) -> Result<(), ValidationErrors> {
        let issues = validate_profile(self);
        if issues.is_empty() {
            Ok(())
        } else {
            Err(ValidationErrors(issues))
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProgrammingLanguage {
    Auto,
    #[serde(rename = "ask_user")]
    AskUser,
    Rust,
    #[serde(rename = "C#")]
    CSharp,
    Python,
    #[serde(rename = "C++")]
    Cpp,
    Go,
    Java,
    TypeScript,
}

impl ProgrammingLanguage {
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::AskUser => "Kullanıcı seçimi bekleniyor",
            Self::Rust => "Rust",
            Self::CSharp => "C#",
            Self::Python => "Python",
            Self::Cpp => "C++",
            Self::Go => "Go",
            Self::Java => "Java",
            Self::TypeScript => "TypeScript",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum UiFramework {
    Auto,
    None,
    Cli,
    Slint,
    Wpf,
    WinUi3,
    Avalonia,
    Qt,
    Gtk,
    Tauri,
    JavaFx,
    React,
    Vue,
    Web,
}

impl UiFramework {
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::None => "None",
            Self::Cli => "CLI",
            Self::Slint => "Slint",
            Self::Wpf => "WPF",
            Self::WinUi3 => "WinUI 3",
            Self::Avalonia => "Avalonia",
            Self::Qt => "Qt",
            Self::Gtk => "GTK",
            Self::Tauri => "Tauri",
            Self::JavaFx => "JavaFX",
            Self::React => "React",
            Self::Vue => "Vue",
            Self::Web => "Web",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Database {
    Auto,
    None,
    SQLite,
    SqlServer,
    PostgreSQL,
}

impl Database {
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::None => "None",
            Self::SQLite => "SQLite",
            Self::SqlServer => "SQL Server",
            Self::PostgreSQL => "PostgreSQL",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct FeaturePreferences {
    pub low_idle_cpu: bool,
    pub low_memory: bool,
    pub gpu_acceleration: bool,
    pub i18n: bool,
    pub default_locale: String,
    pub supported_locales: Vec<String>,
    pub theme: Theme,
    pub tests_required: bool,
    pub logging: bool,
    pub log_level: LogLevel,
}

impl Default for FeaturePreferences {
    fn default() -> Self {
        Self {
            low_idle_cpu: true,
            low_memory: true,
            gpu_acceleration: false,
            i18n: true,
            default_locale: "tr-TR".to_owned(),
            supported_locales: vec!["tr-TR".to_owned()],
            theme: Theme::System,
            tests_required: true,
            logging: true,
            log_level: LogLevel::Info,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Theme {
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct CommonStandards {
    pub architecture: Vec<String>,
    pub quality: Vec<String>,
    pub security: Vec<String>,
    pub performance_benchmarks: Vec<String>,
}

impl Default for CommonStandards {
    fn default() -> Self {
        Self {
            architecture: vec![
                "KISS ve DRY ilkelerini uygula; gereksiz soyutlama oluşturma.".to_owned(),
                "SOLID ve Clean Architecture ilkelerini yalnızca bağlam gerektiriyorsa uygula."
                    .to_owned(),
                "Sorumlulukları ayır ve modüler tasarım kullan.".to_owned(),
                "Bağımlılıkları tek yönlü ve modül sınırlarını açık tut.".to_owned(),
                "Çalışan kodu gereksiz yere yeniden yazma; dar kapsamlı değişiklik yap.".to_owned(),
            ],
            quality: vec![
                "Her değişiklikte ilgili otomatik testleri çalıştır.".to_owned(),
                "Hataları yutma; kullanıcıya uygulanabilir ve Türkçe açıklama ver.".to_owned(),
                "Kamuya açık davranışı sürümle ve değişiklikleri belgele.".to_owned(),
            ],
            security: vec![
                "Gizli bilgi, API anahtarı veya kişisel veriyi kaynak koda yazma.".to_owned(),
                "Var olan dosyayı açık onay olmadan değiştirme veya silme.".to_owned(),
            ],
            performance_benchmarks: vec![
                "Boşta CPU kullanımını ölç ve mümkün olan en düşük değeri hedefle.".to_owned(),
                "Bellek kullanımını benchmark ile izle; sonuçları garanti olarak sunma.".to_owned(),
                "Olay tabanlı akışı ve lazy loading yaklaşımını tercih et; gereksiz polling yapma."
                    .to_owned(),
                "Paralel işlem, dosya ve veritabanı erişimini ölçülmüş darboğaza göre optimize et."
                    .to_owned(),
            ],
        }
    }
}

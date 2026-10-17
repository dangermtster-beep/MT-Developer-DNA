use crate::{provider_descriptor, Provider, ProviderDescriptor, RenderedFile};

pub(crate) trait ProviderAdapter: Sync {
    fn descriptor(&self) -> &'static ProviderDescriptor;
    fn provider(&self) -> Provider {
        self.descriptor().provider
    }
    fn primary_path(&self) -> &'static str {
        self.descriptor().primary_path
    }
    fn skill_path(&self) -> &'static str {
        self.descriptor().project_skill_path
    }
    fn invocation(&self) -> &'static str {
        self.descriptor().invocation
    }
    fn provider_note(&self) -> &'static str {
        self.descriptor().note
    }

    fn additional_files(&self) -> Vec<RenderedFile> {
        if self.provider() != Provider::Gemini {
            return Vec::new();
        }
        vec![RenderedFile {
            relative_path: ".gemini/commands/mt-developer-dna.toml".to_owned(),
            contents: concat!(
                "description = \"MT Developer DNA yazılım geliştirme iş akışını başlatır.\"\n",
                "prompt = \"\"\"\n",
                "Activate and follow the `mt-developer-dna` skill for this software task.\n",
                "Treat the project-local `.mt-dna/` directory as the provider-independent memory source.\n",
                "Preserve the user's chosen programming language and the existing architecture.\n",
                "Do not claim unsupported provider capabilities.\n\n",
                "User request:\n{{args}}\n\"\"\"\n"
            ).to_owned(),
        }]
    }

    fn warnings(&self) -> Vec<String> {
        match self.provider() {
            Provider::Gemini => vec!["Gemini CLI skill ve komut keşfi canlı oturumda ayrıca doğrulanmalıdır.".to_owned()],
            Provider::Antigravity => vec!["Antigravity eski workflow mekanizması 19 Ekim 2026'da kaldırılacağı için `.agent/workflows` çıktısı üretilmedi; skill keşfi canlı uygulamada ayrıca doğrulanmalıdır.".to_owned()],
            Provider::Cline => vec!["Cline IDE eklentisi canlı ortamda doğrulanmadı; dosya paketi resmî skill yoluna göre üretildi.".to_owned()],
            _ => Vec::new(),
        }
    }
}

struct RegistryAdapter(Provider);
impl ProviderAdapter for RegistryAdapter {
    fn descriptor(&self) -> &'static ProviderDescriptor {
        provider_descriptor(self.0)
    }
}

static CODEX: RegistryAdapter = RegistryAdapter(Provider::Codex);
static CLAUDE: RegistryAdapter = RegistryAdapter(Provider::Claude);
static GEMINI: RegistryAdapter = RegistryAdapter(Provider::Gemini);
static ANTIGRAVITY: RegistryAdapter = RegistryAdapter(Provider::Antigravity);
static CURSOR: RegistryAdapter = RegistryAdapter(Provider::Cursor);
static CLINE: RegistryAdapter = RegistryAdapter(Provider::Cline);
static OPENCODE: RegistryAdapter = RegistryAdapter(Provider::OpenCode);

pub(crate) fn adapter_for(provider: Provider) -> &'static dyn ProviderAdapter {
    match provider {
        Provider::Codex => &CODEX,
        Provider::Claude => &CLAUDE,
        Provider::Gemini => &GEMINI,
        Provider::Antigravity => &ANTIGRAVITY,
        Provider::Cursor => &CURSOR,
        Provider::Cline => &CLINE,
        Provider::OpenCode => &OPENCODE,
    }
}

use crate::domain::{AdapterKind, Architecture, NewProvider, ProviderConfig};
use anyhow::{anyhow, Context, Result};
use keyring::Entry;
use serde::Serialize;
use std::{fs, path::PathBuf, sync::Mutex};

const SERVICE: &str = "reciprocal";

#[derive(Clone)]
pub struct AppState {
    providers: std::sync::Arc<Mutex<Vec<ProviderConfig>>>,
    config_path: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderView {
    pub id: String,
    pub name: String,
    pub architecture: Architecture,
    pub adapter: AdapterKind,
    pub endpoint: String,
    pub model: String,
    pub configured: bool,
}

impl AppState {
    pub fn load() -> Self {
        let mut path = dirs::config_dir().unwrap_or_else(std::env::temp_dir);
        path.push("Reciprocal");
        let _ = fs::create_dir_all(&path);
        path.push("providers.json");
        let providers = fs::read_to_string(&path)
            .ok().and_then(|s| serde_json::from_str::<Vec<ProviderConfig>>(&s).ok())
            .unwrap_or_default();
        Self { providers: std::sync::Arc::new(Mutex::new(providers)), config_path: path }
    }

    fn save(&self) -> Result<()> {
        let providers = self.providers.lock().map_err(|_| anyhow!("provider store is locked"))?;
        let data = serde_json::to_vec_pretty(&*providers)?;
        fs::write(&self.config_path, data).context("could not save local provider configuration")
    }

    pub fn provider_views(&self) -> Vec<ProviderView> {
        self.providers.lock().map(|items| items.iter().map(|p| ProviderView {
            id: p.id.clone(), name: p.name.clone(), architecture: p.architecture,
            adapter: p.adapter, endpoint: p.endpoint.clone(), model: p.model.clone(),
            configured: p.secret_id.as_ref().map(|id| Entry::new(SERVICE, id).ok().and_then(|e| e.get_password().ok()).is_some()).unwrap_or(p.adapter == AdapterKind::CodexCli),
        }).collect()).unwrap_or_default()
    }

    pub fn add_provider(&self, input: NewProvider) -> Result<ProviderView> {
        if input.name.trim().is_empty() || input.model.trim().is_empty() {
            return Err(anyhow!("اسم الموصل ومعرّف النموذج مطلوبان"));
        }
        if input.adapter == AdapterKind::CodexCli && input.architecture != Architecture::Autoregressive {
            return Err(anyhow!("Codex CLI يُسجّل ضمن وكلاء AR فقط"));
        }
        let endpoint = input.endpoint.unwrap_or_default();
        if input.adapter == AdapterKind::OpenAiCompatible && endpoint.trim().is_empty() {
            return Err(anyhow!("عنوان API مطلوب"));
        }
        if input.adapter == AdapterKind::OpenAiCompatible && input.api_key.as_deref().unwrap_or("").trim().is_empty() {
            return Err(anyhow!("مفتاح API مطلوب لهذا الموصل"));
        }
        let id = uuid::Uuid::new_v4().to_string();
        let secret_id = if let Some(secret) = input.api_key.filter(|s| !s.trim().is_empty()) {
            Entry::new(SERVICE, &id)?.set_password(&secret).context("تعذر حفظ المفتاح في مدير كلمات مرور النظام")?;
            Some(id.clone())
        } else { None };
        let config = ProviderConfig {
            id: id.clone(), name: input.name, architecture: input.architecture,
            adapter: input.adapter, endpoint, model: input.model, secret_id,
        };
        self.providers.lock().map_err(|_| anyhow!("provider store is locked"))?.push(config.clone());
        self.save()?;
        Ok(ProviderView { id, name: config.name, architecture: config.architecture,
            adapter: config.adapter, endpoint: config.endpoint, model: config.model, configured: true })
    }

    pub fn remove_provider(&self, id: &str) -> Result<()> {
        let mut providers = self.providers.lock().map_err(|_| anyhow!("provider store is locked"))?;
        let index = providers.iter().position(|p| p.id == id).ok_or_else(|| anyhow!("الموصل غير موجود"))?;
        let removed = providers.remove(index);
        if removed.secret_id.is_some() {
            if let Ok(entry) = Entry::new(SERVICE, id) { let _ = entry.delete_credential(); }
        }
        drop(providers);
        self.save()
    }

    pub fn require_provider(&self, id: Option<&str>, architecture: Architecture) -> Result<ProviderConfig> {
        let id = id.ok_or_else(|| anyhow!("اختر موصل {} أولاً", architecture.label()))?;
        let providers = self.providers.lock().map_err(|_| anyhow!("provider store is locked"))?;
        let provider = providers.iter().find(|p| p.id == id).ok_or_else(|| anyhow!("الموصل المحدد غير موجود"))?;
        if provider.architecture != architecture {
            return Err(anyhow!("رفض الإسناد: هذا الموصل ليس من فئة {}", architecture.label()));
        }
        Ok(provider.clone())
    }

    pub async fn invoke(&self, provider: &ProviderConfig, prompt: &str) -> Result<String> {
        match provider.adapter {
            AdapterKind::CodexCli => {
                let output = tokio::process::Command::new("codex")
                    .arg("exec").arg("--skip-git-repo-check").arg(prompt)
                    .output().await.context("لم يتم العثور على Codex CLI. ثبّته وسجّل الدخول أولاً")?;
                if !output.status.success() {
                    return Err(anyhow!("Codex CLI failed: {}", String::from_utf8_lossy(&output.stderr)));
                }
                Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
            }
            AdapterKind::OpenAiCompatible => {
                let secret_id = provider.secret_id.as_deref().ok_or_else(|| anyhow!("مفتاح API غير محفوظ لهذا الموصل"))?;
                let key = Entry::new(SERVICE, secret_id)?.get_password().context("مفتاح API غير متاح في مدير كلمات المرور")?;
                let endpoint = format!("{}/chat/completions", provider.endpoint.trim_end_matches('/'));
                let response = reqwest::Client::new().post(endpoint).bearer_auth(key)
                    .json(&serde_json::json!({
                        "model": provider.model,
                        "messages": [{"role":"user","content":prompt}],
                        "stream": false
                    })).send().await.context("فشل الاتصال بمزوّد النموذج")?;
                let status = response.status();
                let body: serde_json::Value = response.json().await.context("استجابة API غير صالحة")?;
                if !status.is_success() {
                    return Err(anyhow!("Provider returned {status}: {}", body.get("error").unwrap_or(&body)));
                }
                body.pointer("/choices/0/message/content").and_then(|v| v.as_str())
                    .map(str::to_owned).ok_or_else(|| anyhow!("لم أجد نصاً في استجابة النموذج: {body}"))
            }
        }
    }
}

use crate::domain::{AdapterKind, Architecture, NewProvider, ProviderConfig, ToolEvent};
use crate::workspace::WorkspaceState;
use anyhow::{anyhow, Context, Result};
use keyring::Entry;
use serde::Serialize;
use std::{fs, path::PathBuf, sync::{Arc, Mutex}};
use serde_json::{json, Value};

const SERVICE: &str = "reciprocal";
const MAX_TOOL_ROUNDS: usize = 8;

#[derive(Clone)]
pub struct AppState {
    providers: Arc<Mutex<Vec<ProviderConfig>>>,
    config_path: PathBuf,
    pub workspace: Arc<WorkspaceState>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderView {
    pub id: String, pub name: String, pub architecture: Architecture, pub adapter: AdapterKind,
    pub endpoint: String, pub model: String, pub configured: bool, pub supports_tools: bool,
}

pub struct AgentOutput {
    pub output: String,
    pub tool_events: Vec<ToolEvent>,
}

impl AppState {
    pub fn load() -> Self {
        let mut path = dirs::config_dir().unwrap_or_else(std::env::temp_dir);
        path.push("Reciprocal");
        let _ = fs::create_dir_all(&path);
        path.push("providers.json");
        let providers = fs::read_to_string(&path).ok()
            .and_then(|s| serde_json::from_str::<Vec<ProviderConfig>>(&s).ok()).unwrap_or_default();
        Self { providers: Arc::new(Mutex::new(providers)), config_path: path, workspace: Arc::new(WorkspaceState::default()) }
    }

    fn save(&self) -> Result<()> {
        let providers = self.providers.lock().map_err(|_| anyhow!("provider store is locked"))?;
        fs::write(&self.config_path, serde_json::to_vec_pretty(&*providers)?)
            .context("could not save local provider configuration")
    }

    pub fn provider_views(&self) -> Vec<ProviderView> {
        self.providers.lock().map(|items| items.iter().map(|p| ProviderView {
            id:p.id.clone(), name:p.name.clone(), architecture:p.architecture, adapter:p.adapter,
            endpoint:p.endpoint.clone(), model:p.model.clone(),
            configured:p.secret_id.as_ref().map(|id| Entry::new(SERVICE,id).ok().and_then(|e|e.get_password().ok()).is_some()).unwrap_or(p.adapter==AdapterKind::CodexCli),
            supports_tools:p.supports_tools,
        }).collect()).unwrap_or_default()
    }

    pub fn add_provider(&self, input: NewProvider) -> Result<ProviderView> {
        if input.name.trim().is_empty() || input.model.trim().is_empty() { return Err(anyhow!("اسم الموصل ومعرّف النموذج مطلوبان")); }
        if input.adapter == AdapterKind::CodexCli && input.architecture != Architecture::Autoregressive {
            return Err(anyhow!("Codex CLI يُسجّل ضمن وكلاء AR فقط"));
        }
        if input.supports_tools && (input.adapter != AdapterKind::OpenAiCompatible || input.architecture != Architecture::Autoregressive) {
            return Err(anyhow!("أدوات Reciprocal متاحة حالياً فقط لموصل OpenAI-compatible ضمن LLMs Agent"));
        }
        let endpoint=input.endpoint.unwrap_or_default();
        if input.adapter==AdapterKind::OpenAiCompatible && endpoint.trim().is_empty() { return Err(anyhow!("عنوان API مطلوب")); }
        if input.adapter==AdapterKind::OpenAiCompatible && input.api_key.as_deref().unwrap_or("").trim().is_empty() { return Err(anyhow!("مفتاح API مطلوب لهذا الموصل")); }
        let id=uuid::Uuid::new_v4().to_string();
        let secret_id=if let Some(secret)=input.api_key.filter(|s|!s.trim().is_empty()) {
            Entry::new(SERVICE,&id)?.set_password(&secret).context("تعذر حفظ المفتاح في مدير كلمات مرور النظام")?; Some(id.clone())
        } else { None };
        let config=ProviderConfig{id:id.clone(),name:input.name,architecture:input.architecture,adapter:input.adapter,endpoint,model:input.model,secret_id,supports_tools:input.supports_tools};
        self.providers.lock().map_err(|_|anyhow!("provider store is locked"))?.push(config.clone());
        self.save()?;
        Ok(ProviderView{id,name:config.name,architecture:config.architecture,adapter:config.adapter,endpoint:config.endpoint,model:config.model,configured:true,supports_tools:config.supports_tools})
    }

    pub fn remove_provider(&self,id:&str)->Result<()> {
        let mut providers=self.providers.lock().map_err(|_|anyhow!("provider store is locked"))?;
        let index=providers.iter().position(|p|p.id==id).ok_or_else(||anyhow!("الموصل غير موجود"))?;
        let removed=providers.remove(index);
        if removed.secret_id.is_some(){if let Ok(entry)=Entry::new(SERVICE,id){let _=entry.delete_credential();}}
        drop(providers); self.save()
    }

    pub fn require_provider(&self,id:Option<&str>,architecture:Architecture)->Result<ProviderConfig>{
        let id=id.ok_or_else(||anyhow!("اختر موصل {} أولاً",architecture.label()))?;
        let providers=self.providers.lock().map_err(|_|anyhow!("provider store is locked"))?;
        let provider=providers.iter().find(|p|p.id==id).ok_or_else(||anyhow!("الموصل المحدد غير موجود"))?;
        if provider.architecture!=architecture{return Err(anyhow!("رفض الإسناد: هذا الموصل ليس من فئة {}",architecture.label()));}
        Ok(provider.clone())
    }

    async fn api_request(&self, provider:&ProviderConfig, messages:&[Value], use_tools:bool)->Result<Value>{
        let secret_id=provider.secret_id.as_deref().ok_or_else(||anyhow!("مفتاح API غير محفوظ لهذا الموصل"))?;
        let key=Entry::new(SERVICE,secret_id)?.get_password().context("مفتاح API غير متاح في مدير كلمات المرور")?;
        let endpoint=format!("{}/chat/completions",provider.endpoint.trim_end_matches('/'));
        let mut body=json!({"model":provider.model,"messages":messages,"stream":false});
        if use_tools { body["tools"]=workspace_tool_specs(); body["tool_choice"]=json!("auto"); }
        let response=reqwest::Client::new().post(endpoint).bearer_auth(key).json(&body).send().await.context("فشل الاتصال بمزوّد النموذج")?;
        let status=response.status(); let body:Value=response.json().await.context("استجابة API غير صالحة")?;
        if !status.is_success(){return Err(anyhow!("Provider returned {status}: {}",body.get("error").unwrap_or(&body)));}
        Ok(body)
    }

    pub async fn invoke(&self,provider:&ProviderConfig,prompt:&str)->Result<String>{
        if provider.supports_tools { return Ok(self.invoke_agent(provider,prompt,false).await?.output); }
        match provider.adapter {
            AdapterKind::CodexCli=>{
                let output=tokio::process::Command::new("codex").arg("exec").arg("--skip-git-repo-check").arg("--model").arg(&provider.model).arg(prompt)
                    .output().await.context("لم يتم العثور على Codex CLI. ثبّته وسجّل الدخول أولاً")?;
                if !output.status.success(){return Err(anyhow!("Codex CLI failed: {}",String::from_utf8_lossy(&output.stderr)));}
                Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
            }
            AdapterKind::OpenAiCompatible=>{
                let body=self.api_request(provider,&[json!({"role":"user","content":prompt})],false).await?;
                body.pointer("/choices/0/message/content").and_then(Value::as_str).map(str::to_owned)
                    .ok_or_else(||anyhow!("لم أجد نصاً في استجابة النموذج: {body}"))
            }
        }
    }

    pub async fn invoke_agent(&self,provider:&ProviderConfig,prompt:&str,allow_writes:bool)->Result<AgentOutput>{
        if !provider.supports_tools || provider.adapter!=AdapterKind::OpenAiCompatible || provider.architecture!=Architecture::Autoregressive {
            return Ok(AgentOutput{output:self.invoke(provider,prompt).await?,tool_events:vec![]});
        }
        let mut messages=vec![json!({"role":"user","content":prompt})];
        let mut events=Vec::new();
        for _ in 0..MAX_TOOL_ROUNDS {
            let body=self.api_request(provider,&messages,true).await?;
            let message=body.pointer("/choices/0/message").cloned().ok_or_else(||anyhow!("استجابة النموذج لا تحتوي message"))?;
            let calls=message.get("tool_calls").and_then(Value::as_array).cloned().unwrap_or_default();
            if calls.is_empty(){
                let output=message.get("content").and_then(Value::as_str).unwrap_or("").to_owned();
                if output.is_empty(){return Err(anyhow!("النموذج أنهى الطلب دون نص أو استدعاء أداة"));}
                return Ok(AgentOutput{output,tool_events:events});
            }
            messages.push(message);
            for call in calls {
                let id=call.get("id").and_then(Value::as_str).unwrap_or("").to_owned();
                let name=call.pointer("/function/name").and_then(Value::as_str).unwrap_or("").to_owned();
                let raw=call.pointer("/function/arguments").and_then(Value::as_str).unwrap_or("{}");
                let args:Value=serde_json::from_str(raw).unwrap_or_else(|_|json!({}));
                let (output,succeeded)=self.execute_workspace_tool(&name,&args,allow_writes);
                events.push(ToolEvent{name:name.clone(),arguments:args,output:output.clone(),succeeded});
                messages.push(json!({"role":"tool","tool_call_id":id,"content":output}));
            }
        }
        Err(anyhow!("تجاوز النموذج الحد الأقصى لدورات الأدوات ({MAX_TOOL_ROUNDS})؛ أوقفنا الحلقة لمنع التنفيذ غير المحدود"))
    }

    fn execute_workspace_tool(&self,name:&str,args:&Value,allow_writes:bool)->(String,bool){
        let path=args.get("path").and_then(Value::as_str).unwrap_or("");
        let result:Result<String,String>=match name {
            "workspace_list"=>self.workspace.list(Some(path)).map(|v|serde_json::to_string_pretty(&v).unwrap_or_default()),
            "workspace_read"=>self.workspace.read(path),
            "workspace_search"=>self.workspace.search(args.get("query").and_then(Value::as_str).unwrap_or("")).map(|v|serde_json::to_string_pretty(&v).unwrap_or_default()),
            "git_status"=>self.workspace.git_status(),
            "git_diff"=>self.workspace.git_diff(),
            "workspace_write" if !allow_writes=>Err("رفض التنفيذ: المستخدم لم يمنح إذناً بتعديل ملفات Workspace لهذه المهمة".into()),
            "workspace_write"=>self.workspace.write(path,args.get("content").and_then(Value::as_str).unwrap_or("")).map(|_|"تمت كتابة الملف بعد منح المستخدم الإذن المسبق لهذه المهمة".into()),
            _=>Err("أداة غير معروفة أو غير مسموح بها".into()),
        };
        match result {Ok(v)=>(v,true),Err(e)=>(format!("Tool error: {e}"),false)}
    }
}

fn workspace_tool_specs()->Value{
    json!([
        {"type":"function","function":{"name":"workspace_list","description":"List files and directories inside the selected workspace. Use relative paths.","parameters":{"type":"object","properties":{"path":{"type":"string","description":"Relative directory path; empty for workspace root"}},"required":["path"],"additionalProperties":false}}},
        {"type":"function","function":{"name":"workspace_read","description":"Read a UTF-8 text file inside the selected workspace.","parameters":{"type":"object","properties":{"path":{"type":"string","description":"Relative file path"}},"required":["path"],"additionalProperties":false}}},
        {"type":"function","function":{"name":"workspace_search","description":"Search text inside workspace files.","parameters":{"type":"object","properties":{"query":{"type":"string"}},"required":["query"],"additionalProperties":false}}},
        {"type":"function","function":{"name":"workspace_write","description":"Write or replace a file inside the selected workspace. Only call when the user explicitly allowed workspace writes for this task.","parameters":{"type":"object","properties":{"path":{"type":"string"},"content":{"type":"string"}},"required":["path","content"],"additionalProperties":false}}},
        {"type":"function","function":{"name":"git_status","description":"Read-only Git status for the selected workspace.","parameters":{"type":"object","properties":{},"additionalProperties":false}}},
        {"type":"function","function":{"name":"git_diff","description":"Read-only unstaged Git diff for the selected workspace.","parameters":{"type":"object","properties":{},"additionalProperties":false}}}
    ])
}

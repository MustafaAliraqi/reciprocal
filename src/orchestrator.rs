use crate::{domain::{Architecture, StageResult, TaskRequest, TaskResult, Workflow}, providers::AppState};
use anyhow::{anyhow, Result};
use uuid::Uuid;

pub async fn execute(state: std::sync::Arc<AppState>, request: TaskRequest) -> Result<TaskResult> {
    let mut stages = Vec::new();
    let mut context = request.task.clone();
    match request.workflow {
        Workflow::Autoregressive => {
            let p = state.require_provider(request.autoregressive_provider.as_deref(), Architecture::Autoregressive)?;
            let output = state.invoke(&p, &context).await?;
            stages.push(StageResult { architecture: p.architecture, provider: p.name, output: output.clone() });
            context = output;
        }
        Workflow::Diffusion => {
            let p = state.require_provider(request.diffusion_provider.as_deref(), Architecture::Diffusion)?;
            let output = state.invoke(&p, &context).await?;
            stages.push(StageResult { architecture: p.architecture, provider: p.name, output: output.clone() });
            context = output;
        }
        Workflow::ArThenDiffusion | Workflow::DiffusionThenAr => {
            let order = if request.workflow == Workflow::ArThenDiffusion {
                vec![(Architecture::Autoregressive, request.autoregressive_provider.as_deref()), (Architecture::Diffusion, request.diffusion_provider.as_deref())]
            } else {
                vec![(Architecture::Diffusion, request.diffusion_provider.as_deref()), (Architecture::Autoregressive, request.autoregressive_provider.as_deref())]
            };
            for (architecture, id) in order {
                let p = state.require_provider(id, architecture)?;
                let prompt = format!("المهمة الأصلية:\n{}\n\nمخرجات المرحلة السابقة (إن وجدت):\n{}", request.task, context);
                context = state.invoke(&p, &prompt).await?;
                stages.push(StageResult { architecture: p.architecture, provider: p.name, output: context.clone() });
            }
        }
        Workflow::Hierarchical => {\n            let ar = state.require_provider(request.autoregressive_provider.as_deref(), Architecture::Autoregressive)?;\n            let df = state.require_provider(request.diffusion_provider.as_deref(), Architecture::Diffusion)?;\n            let plan_prompt = format!("أنت المنسق الرئيسي. قسّم المهمة التالية إلى خطة تنفيذ قصيرة وواضحة، وحدد الناتج المطلوب ومعايير التحقق. لا تنفذ المهمة بعد.\\n\\nالمهمة: {}", request.task);\n            let plan = state.invoke(&ar, &plan_prompt).await?;\n            stages.push(StageResult { architecture: ar.architecture, provider: ar.name.clone(), output: plan.clone() });\n            let work_prompt = format!("نفّذ المهمة التالية وفق خطة المنسق. أخرج النتيجة مباشرة واذكر أي جزء لم تستطع إنجازه.\\n\\nالمهمة: {}\\n\\nالخطة: {}", request.task, plan);\n            let work = state.invoke(&df, &work_prompt).await?;\n            stages.push(StageResult { architecture: df.architecture, provider: df.name.clone(), output: work.clone() });\n            let verify_prompt = format!("تحقق من اكتمال وصحة الناتج مقارنة بالمهمة والخطة. صحح التناقضات الواضحة، ولا تخترع معلومات.\\n\\nالمهمة: {}\\n\\nالخطة: {}\\n\\nالناتج: {}", request.task, plan, work);\n            context = state.invoke(&ar, &verify_prompt).await?;\n            stages.push(StageResult { architecture: ar.architecture, provider: ar.name, output: context.clone() });\n        }\n        Workflow::Parallel => {
            let ar = state.require_provider(request.autoregressive_provider.as_deref(), Architecture::Autoregressive)?;
            let df = state.require_provider(request.diffusion_provider.as_deref(), Architecture::Diffusion)?;
            let (a, d) = tokio::join!(state.invoke(&ar, &request.task), state.invoke(&df, &request.task));
            let a = a?; let d = d?;
            stages.push(StageResult { architecture: ar.architecture, provider: ar.name, output: a.clone() });
            stages.push(StageResult { architecture: df.architecture, provider: df.name, output: d.clone() });
            context = format!("## مخرجات وكيل AR\n{a}\n\n## مخرجات وكيل الانتشار اللغوي\n{d}");
        }
    }
    if stages.is_empty() { return Err(anyhow!("لم يتم تنفيذ أي مرحلة")); }
    Ok(TaskResult { id: Uuid::new_v4(), workflow: request.workflow, output: context, stages })
}

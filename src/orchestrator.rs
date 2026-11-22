use crate::{domain::{Architecture, StageResult, TaskRequest, TaskResult, ToolEvent, Workflow}, providers::AppState};
use anyhow::{anyhow, Result};
use uuid::Uuid;

pub async fn execute(state: std::sync::Arc<AppState>, request: TaskRequest) -> Result<TaskResult> {
    let mut stages=Vec::new();
    let mut tool_events:Vec<ToolEvent>=Vec::new();
    let mut context=request.task.clone();
    match request.workflow {
        Workflow::Autoregressive=>{
            let p=state.require_provider(request.autoregressive_provider.as_deref(),Architecture::Autoregressive)?;
            let result=state.invoke_agent(&p,&context,request.allow_workspace_writes).await?;
            tool_events.extend(result.tool_events); context=result.output;
            stages.push(StageResult{architecture:p.architecture,provider:p.name,output:context.clone()});
        }
        Workflow::Diffusion=>{
            let p=state.require_provider(request.diffusion_provider.as_deref(),Architecture::Diffusion)?;
            context=state.invoke(&p,&context).await?;
            stages.push(StageResult{architecture:p.architecture,provider:p.name,output:context.clone()});
        }
        Workflow::ArThenDiffusion|Workflow::DiffusionThenAr=>{
            let order=if request.workflow==Workflow::ArThenDiffusion{
                vec![(Architecture::Autoregressive,request.autoregressive_provider.as_deref()),(Architecture::Diffusion,request.diffusion_provider.as_deref())]
            }else{
                vec![(Architecture::Diffusion,request.diffusion_provider.as_deref()),(Architecture::Autoregressive,request.autoregressive_provider.as_deref())]
            };
            for (architecture,id) in order{
                let p=state.require_provider(id,architecture)?;
                let prompt=format!("المهمة الأصلية:\n{}\n\nمخرجات المرحلة السابقة (إن وجدت):\n{}",request.task,context);
                if architecture==Architecture::Autoregressive{
                    let result=state.invoke_agent(&p,&prompt,request.allow_workspace_writes).await?;
                    tool_events.extend(result.tool_events); context=result.output;
                }else{context=state.invoke(&p,&prompt).await?;}
                stages.push(StageResult{architecture:p.architecture,provider:p.name,output:context.clone()});
            }
        }
        Workflow::Parallel=>{
            let ar=state.require_provider(request.autoregressive_provider.as_deref(),Architecture::Autoregressive)?;
            let df=state.require_provider(request.diffusion_provider.as_deref(),Architecture::Diffusion)?;
            let (a,d)=tokio::join!(state.invoke_agent(&ar,&request.task,request.allow_workspace_writes),state.invoke(&df,&request.task));
            let a=a?; let d=d?;
            tool_events.extend(a.tool_events);
            stages.push(StageResult{architecture:ar.architecture,provider:ar.name,output:a.output.clone()});
            stages.push(StageResult{architecture:df.architecture,provider:df.name,output:d.clone()});
            context=format!("## مخرجات وكيل AR\n{}\n\n## مخرجات وكيل الانتشار اللغوي\n{}",a.output,d);
        }
        Workflow::Hierarchical=>{
            let ar=state.require_provider(request.autoregressive_provider.as_deref(),Architecture::Autoregressive)?;
            let df=state.require_provider(request.diffusion_provider.as_deref(),Architecture::Diffusion)?;
            let plan_prompt=format!("أنت المنسق الرئيسي. قسّم المهمة التالية إلى خطة تنفيذ قصيرة وواضحة، وحدد الناتج المطلوب ومعايير التحقق. لا تنفذ المهمة بعد.\n\nالمهمة: {}",request.task);
            let plan_result=state.invoke_agent(&ar,&plan_prompt,request.allow_workspace_writes).await?;
            tool_events.extend(plan_result.tool_events); let plan=plan_result.output;
            stages.push(StageResult{architecture:ar.architecture,provider:ar.name.clone(),output:plan.clone()});
            let work_prompt=format!("نفّذ المهمة التالية وفق خطة المنسق. أخرج النتيجة مباشرة واذكر أي جزء لم تستطع إنجازه.\n\nالمهمة: {}\n\nالخطة: {}",request.task,plan);
            let work=state.invoke(&df,&work_prompt).await?;
            stages.push(StageResult{architecture:df.architecture,provider:df.name.clone(),output:work.clone()});
            let verify_prompt=format!("تحقق من اكتمال وصحة الناتج مقارنة بالمهمة والخطة. صحح التناقضات الواضحة، ولا تخترع معلومات.\n\nالمهمة: {}\n\nالخطة: {}\n\nالناتج: {}",request.task,plan,work);
            let verified=state.invoke_agent(&ar,&verify_prompt,request.allow_workspace_writes).await?;
            tool_events.extend(verified.tool_events); context=verified.output;
            stages.push(StageResult{architecture:ar.architecture,provider:ar.name,output:context.clone()});
        }
    }
    if stages.is_empty(){return Err(anyhow!("لم يتم تنفيذ أي مرحلة"));}
    Ok(TaskResult{id:Uuid::new_v4(),workflow:request.workflow,output:context,stages,tool_events})
}

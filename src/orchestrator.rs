use crate::{
    blackboard::Blackboard,
    domain::{Architecture, BlackboardKind, StageResult, TaskRequest, TaskResult, ToolEvent, Workflow},
    providers::AppState,
};
use anyhow::{anyhow, Result};
use uuid::Uuid;

fn stage_prompt(task: &str, board: &Blackboard, instruction: &str) -> String {
    format!(
        "{instruction}\n\nالمهمة الأصلية:\n{task}\n\n{}",
        board.context_for_agent()
    )
}

fn record_stage(
    board: &mut Blackboard,
    stages: &mut Vec<StageResult>,
    architecture: Architecture,
    provider: &str,
    key: &str,
    kind: BlackboardKind,
    output: &str,
) -> Result<()> {
    board.publish(key, kind, architecture, provider, output)?;
    stages.push(StageResult {
        architecture,
        provider: provider.to_owned(),
        output: output.to_owned(),
    });
    Ok(())
}

pub async fn execute(state: std::sync::Arc<AppState>, request: TaskRequest) -> Result<TaskResult> {
    let task_id = Uuid::new_v4();
    let mut board = Blackboard::new(task_id.to_string());
    let mut stages = Vec::new();
    let mut tool_events: Vec<ToolEvent> = Vec::new();
    let mut output = String::new();

    match request.workflow {
        Workflow::Autoregressive => {
            let provider = state.require_provider(
                request.autoregressive_provider.as_deref(),
                Architecture::Autoregressive,
            )?;
            let result = state
                .invoke_agent(&provider, &request.task, request.allow_workspace_writes)
                .await?;
            tool_events.extend(result.tool_events);
            output = result.output;
            record_stage(
                &mut board, &mut stages, provider.architecture, &provider.name,
                "stage.1.output", BlackboardKind::Output, &output,
            )?;
        }
        Workflow::Diffusion => {
            let provider = state.require_provider(
                request.diffusion_provider.as_deref(),
                Architecture::Diffusion,
            )?;
            output = state.invoke(&provider, &request.task).await?;
            record_stage(
                &mut board, &mut stages, provider.architecture, &provider.name,
                "stage.1.output", BlackboardKind::Output, &output,
            )?;
        }
        Workflow::ArThenDiffusion | Workflow::DiffusionThenAr => {
            let (first_arch, first_id, second_arch, second_id) =
                if request.workflow == Workflow::ArThenDiffusion {
                    (
                        Architecture::Autoregressive, request.autoregressive_provider.as_deref(),
                        Architecture::Diffusion, request.diffusion_provider.as_deref(),
                    )
                } else {
                    (
                        Architecture::Diffusion, request.diffusion_provider.as_deref(),
                        Architecture::Autoregressive, request.autoregressive_provider.as_deref(),
                    )
                };

            let first = state.require_provider(first_id, first_arch)?;
            let first_output = if first_arch == Architecture::Autoregressive {
                let result = state
                    .invoke_agent(&first, &request.task, request.allow_workspace_writes)
                    .await?;
                tool_events.extend(result.tool_events);
                result.output
            } else {
                state.invoke(&first, &request.task).await?
            };
            record_stage(
                &mut board, &mut stages, first.architecture, &first.name,
                "stage.1.output", BlackboardKind::Output, &first_output,
            )?;

            let second = state.require_provider(second_id, second_arch)?;
            let prompt = stage_prompt(
                &request.task,
                &board,
                "تابع عمل المرحلة السابقة. استخدم عناصر لوحة المعرفة المشتركة، وميّز بين النتائج المؤكدة والاستنتاجات، ولا تفترض صحة مخرجات الوكيل السابق دون مراجعتها.",
            );
            output = if second_arch == Architecture::Autoregressive {
                let result = state
                    .invoke_agent(&second, &prompt, request.allow_workspace_writes)
                    .await?;
                tool_events.extend(result.tool_events);
                result.output
            } else {
                state.invoke(&second, &prompt).await?
            };
            record_stage(
                &mut board, &mut stages, second.architecture, &second.name,
                "stage.2.output", BlackboardKind::Output, &output,
            )?;
        }
        Workflow::Parallel => {
            let ar = state.require_provider(
                request.autoregressive_provider.as_deref(),
                Architecture::Autoregressive,
            )?;
            let diffusion = state.require_provider(
                request.diffusion_provider.as_deref(),
                Architecture::Diffusion,
            )?;
            let (ar_result, diffusion_result) = tokio::join!(
                state.invoke_agent(&ar, &request.task, request.allow_workspace_writes),
                state.invoke(&diffusion, &request.task),
            );
            let ar_result = ar_result?;
            let diffusion_output = diffusion_result?;
            tool_events.extend(ar_result.tool_events);

            record_stage(
                &mut board, &mut stages, ar.architecture, &ar.name,
                "parallel.autoregressive", BlackboardKind::Output, &ar_result.output,
            )?;
            record_stage(
                &mut board, &mut stages, diffusion.architecture, &diffusion.name,
                "parallel.diffusion", BlackboardKind::Output, &diffusion_output,
            )?;
            output = format!(
                "## مخرجات وكيل AR\n{}\n\n## مخرجات وكيل الانتشار اللغوي\n{}",
                ar_result.output, diffusion_output
            );
            board.publish(
                "parallel.combined", BlackboardKind::Observation,
                Architecture::Autoregressive, "Reciprocal Orchestrator", &output,
            )?;
        }
        Workflow::Hierarchical => {
            let ar = state.require_provider(
                request.autoregressive_provider.as_deref(),
                Architecture::Autoregressive,
            )?;
            let diffusion = state.require_provider(
                request.diffusion_provider.as_deref(),
                Architecture::Diffusion,
            )?;

            let plan_prompt = format!(
                "أنت المنسق الرئيسي. قسّم المهمة إلى خطة تنفيذ قصيرة، وحدد الناتج المطلوب ومعايير التحقق. لا تنفذ المهمة بعد.\n\nالمهمة: {}",
                request.task
            );
            let plan_result = state
                .invoke_agent(&ar, &plan_prompt, request.allow_workspace_writes)
                .await?;
            tool_events.extend(plan_result.tool_events);
            let plan = plan_result.output;
            record_stage(
                &mut board, &mut stages, ar.architecture, &ar.name,
                "hierarchy.plan", BlackboardKind::Plan, &plan,
            )?;

            let work_prompt = stage_prompt(
                &request.task,
                &board,
                "أنت وكيل التنفيذ. نفّذ المهمة وفق خطة المنسق. أخرج النتيجة مباشرة واذكر ما لم تستطع إنجازه.",
            );
            let work = state.invoke(&diffusion, &work_prompt).await?;
            record_stage(
                &mut board, &mut stages, diffusion.architecture, &diffusion.name,
                "hierarchy.work", BlackboardKind::Artifact, &work,
            )?;

            let verify_prompt = stage_prompt(
                &request.task,
                &board,
                "أنت وكيل التحقق. قارن الناتج بالمهمة والخطة، وحدد التناقضات والنواقص، وصحح ما يمكن تصحيحه دون اختراع معلومات.",
            );
            let verified = state
                .invoke_agent(&ar, &verify_prompt, request.allow_workspace_writes)
                .await?;
            tool_events.extend(verified.tool_events);
            output = verified.output;
            record_stage(
                &mut board, &mut stages, ar.architecture, &ar.name,
                "hierarchy.review", BlackboardKind::Review, &output,
            )?;
        }
    }

    if stages.is_empty() {
        return Err(anyhow!("لم يتم تنفيذ أي مرحلة"));
    }
    Ok(TaskResult {
        id: task_id,
        workflow: request.workflow,
        output,
        stages,
        tool_events,
        blackboard: board.entries(),
    })
}

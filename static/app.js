const $=s=>document.querySelector(s);
let providers=[];
const workflow=$("#workflow");
const arField=$("#ar-field"),diffField=$("#diffusion-field");
function updateWorkflow(){const v=workflow.value;arField.classList.toggle("hidden",v==="diffusion");diffField.classList.toggle("hidden",v==="autoregressive")}
workflow.addEventListener("change",updateWorkflow);
function options(select,architecture){const list=providers.filter(p=>p.architecture===architecture);select.innerHTML='<option value="">اختر موصلاً…</option>';for(const p of list){const o=document.createElement("option");o.value=p.id;o.textContent=p.name+" · "+p.model;select.append(o)}}
function providerList(target,architecture){const el=$(target);el.innerHTML="";const list=providers.filter(p=>p.architecture===architecture);if(!list.length){el.textContent="لا توجد موصلات مضافة بعد.";return}for(const p of list){const row=document.createElement("div");row.className="provider-item";const label=document.createElement("span");label.textContent=p.name+" · "+p.model+(p.supports_tools?" · Workspace tools":"")+(p.configured?"":" · يحتاج إعداداً");const del=document.createElement("button");del.textContent="إزالة";del.onclick=async()=>{await fetch("/api/providers/"+p.id,{method:"DELETE"});await refreshProviders()};row.append(label,del);el.append(row)}}
async function refreshProviders(){const r=await fetch("/api/providers");providers=await r.json();options($("#ar-provider"),"autoregressive");options($("#diffusion-provider"),"diffusion");providerList("#ar-list","autoregressive");providerList("#diffusion-list","diffusion")}
document.querySelectorAll(".nav").forEach(b=>b.addEventListener("click",()=>{document.querySelectorAll(".nav").forEach(x=>x.classList.remove("active"));b.classList.add("active");const view=b.dataset.view;$("#work-view").classList.toggle("hidden",view!=="work");$("#providers-view").classList.toggle("hidden",view!=="providers");$("#page-title").textContent=view==="work"?"تنفيذ مهمة":"موصلات الوكلاء"}));
$("#p-adapter").addEventListener("change",()=>{const codex=$("#p-adapter").value==="codex_cli";$("#endpoint-field").classList.toggle("hidden",codex);if(codex)$("#p-architecture").value="autoregressive"});
$("#p-architecture").addEventListener("change",()=>{if($("#p-architecture").value==="diffusion"&&$("#p-adapter").value==="codex_cli")$("#p-adapter").value="open_ai_compatible"});
$("#provider-form").addEventListener("submit",async e=>{e.preventDefault();const payload={name:$("#p-name").value,architecture:$("#p-architecture").value,adapter:$("#p-adapter").value,endpoint:$("#p-endpoint").value||null,model:$("#p-model").value,api_key:$("#p-key").value||null,supports_tools:$("#p-tools").checked};const r=await fetch("/api/providers",{method:"POST",headers:{"content-type":"application/json"},body:JSON.stringify(payload)});if(!r.ok){alert(await r.text());return}e.target.reset();$("#p-adapter").value="open_ai_compatible";$("#endpoint-field").classList.remove("hidden");await refreshProviders()});
$("#run").addEventListener("click",async()=>{const button=$("#run"),status=$("#status");button.disabled=true;status.textContent="جارٍ تنفيذ المهمة…";$("#result").classList.add("hidden");const payload={task:$("#task").value,workflow:workflow.value,autoregressive_provider:$("#ar-provider").value||null,diffusion_provider:$("#diffusion-provider").value||null,allow_workspace_writes:$("#allow-workspace-writes").checked};try{const r=await fetch("/api/tasks",{method:"POST",headers:{"content-type":"application/json"},body:JSON.stringify(payload)});if(!r.ok)throw new Error(await r.text());const data=await r.json();$("#run-id").textContent=data.id;$("#stages").innerHTML="";for(const s of data.stages){const box=document.createElement("section");box.className="stage";const head=document.createElement("div");head.className="stage-head";head.textContent=(s.architecture==="autoregressive"?"LLMs Agent · ":"DIFFUSION AGENT · ")+s.provider;const pre=document.createElement("pre");pre.textContent=s.output;box.append(head,pre);$("#stages").append(box)}$("#final-output").textContent=data.output;const eventBox=$("#tool-events"),eventList=$("#tool-event-list");eventList.innerHTML="";eventBox.classList.toggle("hidden",!data.tool_events?.length);for(const ev of (data.tool_events||[])){const item=document.createElement("div");item.className="tool-event";const head=document.createElement("strong");head.textContent=ev.name+(ev.succeeded?" · تم":" · رُفض/فشل");const pre=document.createElement("pre");pre.textContent=JSON.stringify(ev.arguments,null,2)+"\\n"+ev.output;item.append(head,pre);eventList.append(item)}$("#result").classList.remove("hidden");status.textContent="اكتمل التنفيذ"}catch(e){status.textContent="تعذّر التنفيذ: "+e.message}finally{button.disabled=false}});

async function refreshWorkspace(){
  try {
    const r=await fetch("/api/workspace");
    const w=await r.json();
    $("#workspace-status").textContent=w.configured?"Workspace: "+w.root:"لم يتم تحديد مجلد عمل.";
  } catch(e) {
    $("#workspace-status").textContent="تعذر قراءة إعداد Workspace";
  }
}
$("#set-workspace").addEventListener("click",async()=>{
  const path=$("#workspace-path").value.trim();
  if(!path){$("#workspace-status").textContent="أدخل مسار المجلد أولاً.";return;}
  const r=await fetch("/api/workspace",{method:"POST",headers:{"content-type":"application/json"},body:JSON.stringify({path})});
  if(!r.ok){$("#workspace-status").textContent="تعذر التعيين: "+await r.text();return;}
  const w=await r.json();
  $("#workspace-status").textContent="Workspace: "+w.root;
});

updateWorkflow();refreshProviders();refreshWorkspace();

function updateToolControls(){
 const architecture=$("#p-architecture").value, adapter=$("#p-adapter").value;
 const allowed=architecture==="autoregressive"&&adapter==="open_ai_compatible";
 $("#p-tools").disabled=!allowed;
 $("#p-tools").closest(".consent").classList.toggle("hidden",!allowed);
 if(!allowed)$("#p-tools").checked=false;
}
$("#p-architecture").addEventListener("change",updateToolControls);
$("#p-adapter").addEventListener("change",updateToolControls);
updateToolControls();

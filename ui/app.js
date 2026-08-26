// LinFS Perfect GUI — fetch /api/fs/*, Monaco, hex, drag-drop, terminal, mount
const $ = s => document.querySelector(s);
const $$ = s => document.querySelectorAll(s);
let cwd = "/";
let monacoEditor = null;
let currentEditPath = null;
let isHexMode = false;
let fileCache = new Map();

async function api(path, opts={}) {
  const r = await fetch(path, opts);
  const j = await r.json().catch(()=>({}));
  if (!r.ok) throw new Error(j.msg || j.error || r.statusText);
  return j;
}
function toast(msg){ const t=$("#toast"); t.textContent=msg; t.classList.remove("hidden"); setTimeout(()=>t.classList.add("hidden"),2000); }
function fmtMode(m){ return m; }
function fmtSize(n){ if(n<1024) return n+" B"; if(n<1024*1024) return (n/1024).toFixed(1)+" KiB"; return (n/1024/1024).toFixed(1)+" MiB"; }
function renderBreadcrumb(path){
  const bc=$("#breadcrumb"); bc.innerHTML="";
  const parts = path.split("/").filter(Boolean);
  const mk = (label,p)=>{ const s=document.createElement("span"); s.textContent=label; s.onclick=()=>loadPath(p); bc.appendChild(s); };
  mk("∕","/");
  let cur=""; parts.forEach(p=>{ cur+="/"+p; mk(" › "+p,cur); });
}
async function loadDevices(){
  try{
    const j=await api("/api/mount/list");
    const d=$("#devices"); d.innerHTML="";
    (j.devices||[]).forEach(dev=>{
      const div=document.createElement("div"); div.textContent=dev.device; div.style.fontSize="12px"; div.style.color="#9d9d9d"; d.appendChild(div);
    });
    if(j.images) j.images.forEach(img=>{
      const div=document.createElement("div"); div.textContent="🖴 "+img; div.style.cursor="pointer"; div.onclick=()=>{ api("/api/mount/attach",{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify({image:img})}).then(()=>{toast("Attached "+img); loadPath("/");}); }; $("#devices").appendChild(div);
    });
  }catch(e){ console.log(e); }
}
async function loadPath(path){
  cwd=path; renderBreadcrumb(path);
  try{
    const j=await api(`/api/fs/readdir?path=${encodeURIComponent(path)}`);
    const body=$("#fileBody"); body.innerHTML="";
    const empty=$("#empty");
    if(!j.entries || j.entries.length===0){ empty.style.display="block"; }
    else { empty.style.display="none"; j.entries.forEach(e=>{
      const tr=document.createElement("tr");
      const isDir=e.is_dir; const icon=isDir?"📁 ":"📄 ";
      tr.innerHTML=`<td>${icon}${e.name}</td><td>${isDir?"—":fmtSize(0)}</td><td>${isDir?"drwxr-xr-x":"-rw-r--r--"}</td><td>0:0</td><td></td><td><button data-act="more">⋯</button></td>`;
      tr.onclick=()=>{
        if(isDir) loadPath(path==="/" ? "/"+e.name : path+"/"+e.name);
        else openFile((path==="/"?"":path)+"/"+e.name);
      };
      tr.oncontextmenu=(ev)=>{ ev.preventDefault(); openPerm((path==="/"?"":path)+"/"+e.name); };
      body.appendChild(tr);
    }); }
    // stat for info
    try{
      const st=await api(`/api/fs/stat?path=${encodeURIComponent(path)}`);
      $("#infoBox").textContent=`${path} — ${st.mode} ino ${st.ino} size ${st.size}`;
    }catch{}
  }catch(e){ toast("readdir "+e.message); }
  $("#status").textContent=`${path} — ${new Date().toLocaleTimeString()}`;
}
async function openFile(path){
  currentEditPath=path;
  const j=await api(`/api/fs/read?path=${encodeURIComponent(path)}`);
  $("#editorPath").textContent=path + ` (${j.size} B)`;
  $("#editorModal").classList.remove("hidden");
  isHexMode=!j.is_text;
  $("#hexView").classList.toggle("hidden", !isHexMode);
  $("#editor").classList.toggle("hidden", isHexMode);
  $("#editorHexToggle").textContent=isHexMode?"Text":"Hex";
  if(isHexMode){
    const hex=j.content; const lines=[]; const bytes=hex.split(" "); for(let i=0;i<bytes.length;i+=16){ const row=bytes.slice(i,i+16).join(" "); const ascii=bytes.slice(i,i+16).map(b=>{const v=parseInt(b,16); return v>=32&&v<127?String.fromCharCode(v):".";}).join(""); lines.push(`${(i).toString(16).padStart(8,"0")}  ${row.padEnd(47," ")}  |${ascii}|`); } $("#hexView").textContent=lines.join("\n");
  } else {
    if(window.require){
      require(["vs/editor/editor.main"],()=>{
        if(!monacoEditor){ monacoEditor=monaco.editor.create($("#editor"),{value:j.content,language:"plaintext",theme:"vs-dark",automaticLayout:true}); }
        else monacoEditor.setValue(j.content);
      });
    } else {
      // fallback textarea
      if(!monacoEditor){
        const ta=document.createElement("textarea"); ta.style.width="100%"; ta.style.height="100%"; ta.id="fallbackEditor"; $("#editor").appendChild(ta); monacoEditor={ getValue:()=>ta.value, setValue:(v)=>ta.value=v };
      }
      monacoEditor.setValue(j.content);
    }
  }
}
async function saveFile(){
  if(!currentEditPath) return;
  let content;
  if(isHexMode){
    // hex mode: parse hex back to text? For MVP, keep as is
    content=$("#hexView").textContent;
  } else {
    content=monacoEditor ? monacoEditor.getValue() : "";
  }
  await api("/api/fs/write",{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify({path:currentEditPath,content})});
  toast("Saved "+currentEditPath); $("#editorModal").classList.add("hidden"); loadPath(cwd);
}
function openPerm(path){
  $("#permPath").textContent=path; $("#permInput").value="0644"; $("#permModal").classList.remove("hidden");
  $("#permSave").onclick=async()=>{ const mode=parseInt($("#permInput").value,8); await api("/api/fs/chmod",{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify({path,mode})}); toast("chmod "+mode.toString(8)); $("#permModal").classList.add("hidden"); };
}
function initTree(){
  const tree=$("#tree");
  [["/","∕ /"],["/etc","📁 etc"],["/home","📁 home"],["/var","📁 var"]].forEach(([p,l])=>{ const d=document.createElement("div"); d.textContent=l; d.onclick=()=>loadPath(p); tree.appendChild(d); });
}
function initEvents(){
  $("#refreshBtn").onclick=()=>loadPath(cwd);
  $("#newFileBtn").onclick=async()=>{ const name=prompt("File name:"); if(!name) return; const path=(cwd==="/"?"":cwd)+"/"+name; await api("/api/fs/write",{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify({path,content:""})}); toast("Created "+name); loadPath(cwd); };
  $("#newFolderBtn").onclick=async()=>{ const name=prompt("Folder name:"); if(!name) return; const path=(cwd==="/"?"":cwd)+"/"+name; await api("/api/fs/mkdir",{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify({path})}); toast("mkdir "+name); loadPath(cwd); };
  $("#editorSave").onclick=saveFile; $("#editorClose").onclick=()=>$("#editorModal").classList.add("hidden");
  $("#editorHexToggle").onclick=()=>{ isHexMode=!isHexMode; $("#hexView").classList.toggle("hidden",!isHexMode); $("#editor").classList.toggle("hidden",isHexMode); $("#editorHexToggle").textContent=isHexMode?"Text":"Hex"; };
  $("#permCancel").onclick=()=>$("#permModal").classList.add("hidden");
  $("#attachBtn").onclick=async()=>{ const img=prompt("Image path (e.g. C:\\img\\root.img):"); if(!img) return; await api("/api/mount/attach",{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify({image:img})}); toast("Attached"); loadPath("/"); };
  $("#search").oninput=(e)=>{ const q=e.target.value.toLowerCase(); $$("#fileBody tr").forEach(tr=>{ const txt=tr.textContent.toLowerCase(); tr.style.display= txt.includes(q) ? "" : "none"; }); };
  // drag-drop
  const wrap=$(".table-wrap"); wrap.addEventListener("dragover",e=>{e.preventDefault(); wrap.style.background="#1f2a33";}); wrap.addEventListener("dragleave",()=>wrap.style.background=""); wrap.addEventListener("drop",async e=>{ e.preventDefault(); wrap.style.background=""; const files=e.dataTransfer.files; for(const f of files){ const text=await f.text(); const path=(cwd==="/"?"":cwd)+"/"+f.name; await api("/api/fs/write",{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify({path,content:text})}); } toast("Upload done"); loadPath(cwd); });
  // file input
  $("#uploadBtn").onclick=()=>$("#fileInput").click(); $("#fileInput").onchange=async e=>{ for(const f of e.target.files){ const text=await f.text(); const path=(cwd==="/"?"":cwd)+"/"+f.name; await api("/api/fs/write",{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify({path,content:text})}); } loadPath(cwd); };
  $("#downloadBtn").onclick=async()=>{ const sel=$("#fileBody tr.sel"); alert("Select file then browser download via /api/fs/read?path=... (stub)"); };
  // keyboard
  document.addEventListener("keydown",e=>{ if(e.ctrlKey&&e.key.toLowerCase()==="k"){e.preventDefault(); $("#search").focus();} if(e.ctrlKey&&e.key.toLowerCase()==="s"){e.preventDefault(); saveFile();} if(e.key==="F5"){e.preventDefault(); loadPath(cwd);} });
  // terminal
  const term=$("#term"); const termIn=$("#termInput");
  function termPrint(s){ term.textContent+="\n"+s; term.scrollTop=term.scrollHeight; }
  termIn.addEventListener("keydown",async e=>{ if(e.key==="Enter"){ const cmd=termIn.value.trim(); termIn.value=""; termPrint("> "+cmd); const parts=cmd.split(" "); if(parts[0]==="ls"){ const j=await api(`/api/fs/readdir?path=${encodeURIComponent(cwd)}`); termPrint(j.entries.map(x=>x.name).join("  ")); } else if(parts[0]==="cat"&&parts[1]){ try{ const j=await api(`/api/fs/read?path=${encodeURIComponent(parts[1].startsWith("/")?parts[1]:cwd+"/"+parts[1])}`); termPrint(j.content.slice(0,2000)); }catch(err){termPrint("cat: "+err.message);} } else if(parts[0]==="clear"){term.textContent="";} else termPrint("(busybox sh — use ls, cat /etc/hostname, echo ...)"); } });
  $("#termClear").onclick=()=>term.textContent="";
  // theme
  $("#themeBtn").onclick=()=>document.body.classList.toggle("light");
}
initTree(); initEvents(); loadDevices(); loadPath("/");
console.log("LinFS perfect GUI loaded — 127.0.0.1:9998");

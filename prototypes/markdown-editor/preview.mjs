import { createEditor } from './editor.mjs';
const body = document.querySelector('#body');
const save = document.querySelector('#save');
const reload = document.querySelector('#reload');
const status = document.querySelector('#status');
async function request(payload) {
  const response = await fetch('/store', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(payload) });
  if (!response.ok) throw new Error(`请求失败 (${response.status})`);
  const result = await response.json();
  if (!result.ok) throw new Error(result.error);
  return result.value;
}
const editor = createEditor({
  read: async (id, revision) => (await request({ operation: 'read', id, revision })).text,
  save: async (id, revision, text) => (await request({ operation: 'save', id, revision, text })).revision,
});
function render() {
  const state = editor.snapshot();
  body.disabled = !state.id;
  if (body.value !== state.text) body.value = state.text;
  save.disabled = !state.id || !state.dirty || state.saving;
  reload.disabled = state.dirty || state.saving;
  status.textContent = state.error || `版本 ${state.revision} · ${state.saving ? '保存中' : state.dirty ? '有未保存修改' : '已保存'}`;
}
body.addEventListener('input', () => { editor.edit(body.value); render(); });
save.addEventListener('click', async () => { const pending = editor.save(); render(); await pending; render(); });
async function load() {
  try { await editor.load('preview-note', editor.snapshot().revision || 1); render(); }
  catch (error) { status.textContent = error.message; }
}
reload.addEventListener('click', load);
await load();

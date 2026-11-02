import { test } from 'node:test';
import assert from 'node:assert/strict';
import { startPreview } from './preview-server.mjs';
test('local preview serves UI and real storage; cross-origin write rejected', async () => {
  const app = await startPreview();
  try {
    assert.equal((await fetch(app.origin)).status, 200);
    const payload = { operation: 'save', id: 'preview-note', revision: 1, text: '# 测试' };
    const headers = { 'Content-Type': 'application/json', Origin: app.origin };
    const blocked = await fetch(`${app.origin}/store`, { method: 'POST', headers: { ...headers, Origin: 'https://example.com' }, body: JSON.stringify(payload) });
    assert.equal(blocked.status, 403);
    const saved = await fetch(`${app.origin}/store`, { method: 'POST', headers, body: JSON.stringify(payload) });
    assert.equal((await saved.json()).value.revision, 2);
    const read = await fetch(`${app.origin}/store`, { method: 'POST', headers, body: JSON.stringify({ operation: 'read', id: 'preview-note', revision: 2 }) });
    assert.equal((await read.json()).value.text, '# 测试');
  } finally { await app.close(); }
});

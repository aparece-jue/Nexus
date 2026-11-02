// Real process/storage integration, separate from the editor's stub-based tests.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createEditor } from './editor.mjs';

const binary = fileURLToPath(new URL(
  `../knowledge-store/target/debug/nexus-knowledge-store-probe${process.platform === 'win32' ? '.exe' : ''}`,
  import.meta.url,
));

function request(root, payload) {
  return new Promise((resolve, reject) => {
    const child = execFile(binary, [root], { timeout: 10000 }, (error, stdout) => {
      if (error) return reject(error);
      try {
        const response = JSON.parse(stdout);
        if (!response.ok) throw new Error(response.error);
        resolve(response.value);
      } catch (failure) { reject(failure); }
    });
    child.stdin.on('error', reject);
    child.stdin.end(JSON.stringify(payload));
  });
}

test('editor saves Unicode Markdown through Rust/SQLite; external change preserves conflicting draft', async () => {
  const root = await mkdtemp(join(tmpdir(), 'nexus-editor-'));
  try {
    await request(root, { operation: 'save', id: 'note', revision: 0, text: '# 电源\nAD5422' });
    const port = {
      read: async (id, revision) => (await request(root, { operation: 'read', id, revision })).text,
      save: async (id, revision, text) => (await request(root, { operation: 'save', id, revision, text })).revision,
    };
    const editor = createEditor(port);
    await editor.load('note', 1);
    assert.equal(editor.snapshot().text, '# 电源\nAD5422');
    editor.edit('# 选型\n- 已评估');
    await editor.save();
    assert.equal(editor.snapshot().error, null);
    assert.equal(editor.snapshot().revision, 2);
    assert.equal(editor.snapshot().dirty, false);
    const reopened = createEditor(port);
    await reopened.load('note', 2);
    assert.equal(reopened.snapshot().text, '# 选型\n- 已评估');
    assert.equal(await port.read('note', 1), '# 电源\nAD5422');
    await port.save('note', 2, '# External version');
    reopened.edit('# 未保存草稿');
    await reopened.save();
    assert.equal(reopened.snapshot().error, 'version conflict');
    assert.equal(reopened.snapshot().text, '# 未保存草稿');
    assert.equal(reopened.snapshot().dirty, true);
    assert.equal(await port.read('note', 3), '# External version');
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

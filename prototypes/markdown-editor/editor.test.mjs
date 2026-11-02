import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createEditor } from './editor.mjs';

test('late load of a different note cannot replace newer selection', async () => {
  const pending = new Map();
  const editor = createEditor({ read: id => new Promise(resolve => pending.set(id, resolve)) });
  const old = editor.load('old', 1);
  const recent = editor.load('recent', 1);
  pending.get('recent')('new selection'); await recent;
  pending.get('old')('old selection'); await old;
  assert.equal(editor.snapshot().id, 'recent');
});

test('save uses loaded version; conflict preserves user text', async () => {
  const calls = [];
  const editor = createEditor({
    read: async () => '# Original',
    save: async (...args) => { calls.push(args); throw new Error('version conflict'); },
  });
  await editor.load('note', 2);
  editor.edit('# My changes');
  await editor.save();
  assert.deepEqual(calls, [['note', 2, '# My changes']]);
  assert.equal(editor.snapshot().text, '# My changes');
  assert.equal(editor.snapshot().revision, 2);
  assert.equal(editor.snapshot().dirty, true);
  assert.equal(editor.snapshot().error, 'version conflict');
  await assert.rejects(editor.load('note', 3), /Unsaved/);
});

test('editing during save preserves new draft and prevents duplicate save', async () => {
  let complete;
  let count = 0;
  const editor = createEditor({
    read: async () => 'v1',
    save: () => { count++; return new Promise(resolve => { complete = resolve; }); },
  });
  await editor.load('note', 1);
  editor.edit('v2');
  const saving = editor.save();
  await editor.save();
  editor.edit('v3');
  complete(2);
  await saving;
  assert.equal(count, 1);
  assert.equal(editor.snapshot().text, 'v3');
  assert.equal(editor.snapshot().revision, 2);
  assert.equal(editor.snapshot().dirty, true);
});

test('successful save is clean and can reload; late load cannot overwrite typing', async () => {
  let complete;
  const editor = createEditor({
    read: () => new Promise(resolve => { complete = resolve; }),
    save: async () => 2,
  });
  const first = editor.load('note', 1);
  complete('v1'); await first;
  editor.edit('v2'); await editor.save();
  assert.equal(editor.snapshot().dirty, false);
  const reload = editor.load('note', 2);
  editor.edit('new draft');
  complete('v2'); await reload;
  assert.equal(editor.snapshot().text, 'new draft');
  assert.equal(editor.snapshot().dirty, true);
});

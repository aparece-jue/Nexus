// @vitest-environment jsdom
import { cleanup, fireEvent, render as renderBase, screen, waitFor } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';

const backend = vi.hoisted(() => ({ notes: new Map<string, { revision: number; text: string; archived?: boolean }>() }));
vi.mock('react-dom/client', () => ({ createRoot: () => ({ render: vi.fn() }) }));
vi.mock('@tauri-apps/api/core', () => ({
  isTauri: () => true,
  invoke: async (command: string, args: any = {}) => {
    if (command === 'list_notes') return [...backend.notes].filter(([, note]) => Boolean(note.archived) === Boolean(args.archived)).map(([id, note]) => ({ id, revision: note.revision, title: note.text.replace(/^# /, '').split('\n')[0] }));
    if (command === 'set_note_archived') {
      const note = backend.notes.get(args.id)!;
      if (note.revision !== args.expectedRevision) throw new Error('version conflict');
      backend.notes.set(args.id, { ...note, revision: note.revision + 1, archived: args.archived });
      return note.revision + 1;
    }
    if (command === 'read_note') {
      const note = backend.notes.get(args.id);
      if (!note) throw new Error('missing note');
      return note;
    }
    if (command === 'save_note') {
      const request = args.request;
      const previous = backend.notes.get(request.id);
      if ((previous?.revision ?? 0) !== request.expected_revision) throw new Error('version conflict');
      backend.notes.set(request.id, { revision: request.expected_revision + 1, text: request.text });
      return request.expected_revision + 1;
    }
    throw new Error(`unexpected command ${command}`);
  },
}));
vi.mock('./SourceEditor', () => ({ SourceEditor: ({ initial, onChange }: { initial: string; onChange: (value: string) => void }) =>
  <textarea aria-label="Markdown 原文" defaultValue={initial} onChange={event => onChange(event.target.value)} /> }));
vi.mock('./MarkdownPreview', () => ({ MarkdownPreview: ({ text }: { text: string }) => <div>{text}</div> }));
import { App } from './src';
function render(ui: React.ReactNode) {
  const view=renderBase(ui);
  fireEvent.click(screen.getByRole('button',{name:'便签'}));
  return view;
}

afterEach(() => { cleanup(); backend.notes.clear(); });

test('archive and restore preserve identity and make the archived editor read only', async () => {
  backend.notes.set('desktop-note', { revision: 1, text: '# Original' });
  render(<App />);
  fireEvent.click(await screen.findByRole('button', { name: '归档 Original' }));
  await screen.findByText('此便签已归档，请在归档列表恢复后编辑。');
  expect(screen.queryByLabelText('Markdown 原文')).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: '查看归档' }));
  fireEvent.click(await screen.findByRole('button', { name: '恢复 Original' }));
  await screen.findByLabelText('Markdown 原文');
  expect(backend.notes.size).toBe(1);
  expect(backend.notes.get('desktop-note')).toEqual({ revision: 3, text: '# Original', archived: false });
});

test('unsaved edits block switching; saving updates only the selected note and survives remount', async () => {
  backend.notes.set('desktop-note', { revision: 1, text: '# First' });
  backend.notes.set('second', { revision: 1, text: '# Second' });
  const view = render(<App />);
  fireEvent.change(await screen.findByLabelText('Markdown 原文'), { target: { value: '# Updated' } });
  expect((screen.getByRole('button', { name: 'Second' }) as HTMLButtonElement).disabled).toBe(true);
  fireEvent.click(screen.getByRole('button', { name: '保存新版本' }));
  await screen.findByRole('button', { name: 'Updated' });
  expect(backend.notes.get('second')?.text).toBe('# Second');
  view.unmount(); render(<App />);
  await waitFor(() => expect((screen.getByLabelText('Markdown 原文') as HTMLTextAreaElement).value).toBe('# Updated'));
});

test('a concurrent edit rejects save and keeps the local draft visible', async () => {
  backend.notes.set('desktop-note', { revision: 1, text: '# Original' });
  render(<App />);
  fireEvent.change(await screen.findByLabelText('Markdown 原文'), { target: { value: '# Local draft' } });
  backend.notes.set('desktop-note', { revision: 2, text: '# Other window' });
  fireEvent.click(screen.getByRole('button', { name: '保存新版本' }));
  await waitFor(() => expect(screen.getByRole('status').textContent).toBe('version conflict'));
  expect((screen.getByLabelText('Markdown 原文') as HTMLTextAreaElement).value).toBe('# Local draft');
  expect(backend.notes.get('desktop-note')?.text).toBe('# Other window');
});

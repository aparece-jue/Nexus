import React, { useEffect, useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { EditorView, keymap, lineNumbers } from '@codemirror/view';
import { EditorState } from '@codemirror/state';
import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';
import { markdown } from '@codemirror/lang-markdown';
import Markdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import rehypeSanitize from 'rehype-sanitize';
import './style.css';

const initialText = '# Nexus 编辑组件验证\n\n- [ ] 中文输入与撤销\n\n| 器件 | 状态 |\n| --- | --- |\n| AD5422 | 待评估 |\n\n```rust\nlet revision = 1;\n```';

function SourceEditor({ initial, onChange }) {
  const host = useRef(null);
  const changed = useRef(onChange);
  changed.current = onChange;
  useEffect(() => {
    const view = new EditorView({
      parent: host.current,
      state: EditorState.create({ doc: initial, extensions: [
        lineNumbers(), history(), markdown(), EditorView.lineWrapping,
        keymap.of([...defaultKeymap, ...historyKeymap]),
        EditorView.contentAttributes.of({ 'aria-label': 'Markdown 原文' }),
        EditorView.updateListener.of(update => {
          if (update.docChanged) changed.current(update.state.doc.toString());
        }),
      ] }),
    });
    return () => view.destroy();
  }, [initial]);
  return <div ref={host} />;
}

function App() {
  const [text, setText] = useState(initialText);
  const [preview, setPreview] = useState(true);
  return <main>
    <h1>Nexus Markdown 组件小样</h1>
    <p>仅验证组件，不保存资料。存储状态与 Tauri 接入尚未连接。</p>
    <button aria-pressed={preview} onClick={() => setPreview(!preview)}>渲染预览</button>
    <div className="panels">
      <section aria-label="编辑区"><SourceEditor initial={initialText} onChange={setText} /></section>
      {preview && <section aria-label="Markdown 预览"><Markdown
        remarkPlugins={[remarkGfm]} rehypePlugins={[rehypeSanitize]} skipHtml
        components={{
          img: () => <span>图片暂未授权加载</span>,
          a: ({ children }) => <span>{children}（链接仅展示）</span>,
        }}
      >{text}</Markdown></section>}
    </div>
  </main>;
}
createRoot(document.getElementById('root')).render(<App />);

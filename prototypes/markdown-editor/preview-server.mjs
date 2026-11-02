// Local UI experiment only. Production uses Tauri, not this HTTP/process bridge.
import { createServer } from 'node:http';
import { execFile } from 'node:child_process';
import { readFile, mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const binary = fileURLToPath(new URL(`../knowledge-store/target/debug/nexus-knowledge-store-probe${process.platform === 'win32' ? '.exe' : ''}`, import.meta.url));
function call(root, request) {
  return new Promise((resolve, reject) => {
    const child = execFile(binary, [root], { timeout: 10000 }, (error, output) => {
      if (error) return reject(error);
      try { resolve(JSON.parse(output)); } catch (failure) { reject(failure); }
    });
    child.stdin.on('error', reject);
    child.stdin.end(JSON.stringify(request));
  });
}

export async function startPreview() {
  const root = await mkdtemp(join(tmpdir(), 'nexus-preview-'));
  let origin;
  const server = createServer(async (req, res) => {
    res.setHeader('Cache-Control', 'no-store');
    res.setHeader('X-Content-Type-Options', 'nosniff');
    res.setHeader('Content-Security-Policy', "default-src 'self'; script-src 'self'; style-src 'self'; frame-ancestors 'none'");
    try {
      if (req.headers.host !== new URL(origin).host) { res.writeHead(403).end(); return; }
      const assets = { '/': ['preview.html', 'text/html'], '/preview.mjs': ['preview.mjs', 'text/javascript'], '/editor.mjs': ['editor.mjs', 'text/javascript'] };
      if (req.method === 'GET' && assets[req.url]) {
        const [name, type] = assets[req.url];
        res.setHeader('Content-Type', `${type}; charset=utf-8`);
        res.end(await readFile(new URL(name, import.meta.url))); return;
      }
      if (req.method !== 'POST' || req.url !== '/store') { res.writeHead(404).end(); return; }
      if (req.headers.origin !== origin || req.headers['content-type'] !== 'application/json') {
        res.writeHead(403).end(); return;
      }
      let body = '';
      for await (const chunk of req) {
        body += chunk;
        if (Buffer.byteLength(body) > 65536) { res.writeHead(413).end(); return; }
      }
      const request = JSON.parse(body);
      if (request.id !== 'preview-note' || !['read', 'save'].includes(request.operation)) {
        res.writeHead(400).end(); return;
      }
      res.setHeader('Content-Type', 'application/json');
      res.end(JSON.stringify(await call(root, request)));
    } catch {
      res.writeHead(500).end(JSON.stringify({ ok: false, error: 'Preview request failed' }));
    }
  });
  await call(root, { operation: 'save', id: 'preview-note', revision: 0, text: '# Nexus\n编辑、保存，再重新读取。' });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  origin = `http://127.0.0.1:${server.address().port}`;
  return { origin, close: async () => {
    await new Promise(resolve => server.close(resolve));
    await rm(root, { recursive: true, force: true });
  } };
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const preview = await startPreview();
  console.log(`临时知识编辑小样：${preview.origin}（不保存真实资料，退出后清理）`);
  process.once('SIGINT', async () => { await preview.close(); process.exit(); });
}

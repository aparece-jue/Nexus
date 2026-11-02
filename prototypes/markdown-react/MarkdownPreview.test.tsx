// @vitest-environment jsdom
import { afterEach, expect, test } from 'vitest';
import { cleanup, render, screen } from '@testing-library/react';
import { MarkdownPreview } from './MarkdownPreview';
afterEach(cleanup);
test('renders GFM tables, tasks and code as content', () => {
  render(<MarkdownPreview text={'# 中文\n\n| 器件 |\n| --- |\n| AD5422 |\n\n- [ ] 审核\n\n```rust\nlet x = 1;\n```'} />);
  expect(screen.getByRole('heading').textContent).toBe('中文');
  expect(screen.getByRole('table').textContent).toContain('AD5422');
  expect(screen.getByRole('checkbox').hasAttribute('disabled')).toBe(true);
  expect(screen.getByText('let x = 1;')).toBeTruthy();
});
test('untrusted content creates no scripts, frames, active links or remote images', () => {
  const { container } = render(<MarkdownPreview text={'<script>alert(1)</script>\n\n<iframe src="https://example.com"></iframe>\n\n[x](javascript:alert)\n\n![remote](https://example.com/pixel)\n\n[外链](https://example.com)'} />);
  expect(container.querySelector('script,iframe,img,a')).toBeNull();
  expect(screen.getByText('图片暂未授权加载')).toBeTruthy();
});

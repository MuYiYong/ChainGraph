#!/usr/bin/env python3
"""
将 Markdown 文档转换为 HTML 格式
"""

import re
import os
from pathlib import Path

def escape_html(text):
    """转义 HTML 特殊字符"""
    return text.replace('&', '&amp;').replace('<', '&lt;').replace('>', '&gt;')

def markdown_to_html(md_content):
    """简单的 Markdown 到 HTML 转换"""
    lines = md_content.split('\n')
    html_lines = []
    in_code_block = False
    code_lang = ""
    in_table = False
    in_list = False
    list_type = None
    
    i = 0
    while i < len(lines):
        line = lines[i]
        
        # 代码块处理
        if line.startswith('```'):
            if not in_code_block:
                in_code_block = True
                code_lang = line[3:].strip()
                lang_class = f' class="language-{code_lang}"' if code_lang else ''
                html_lines.append(f'<pre><code{lang_class}>')
            else:
                in_code_block = False
                html_lines.append('</code></pre>')
            i += 1
            continue
        
        if in_code_block:
            html_lines.append(escape_html(line))
            i += 1
            continue
        
        # 表格处理
        if '|' in line and line.strip().startswith('|'):
            if not in_table:
                in_table = True
                html_lines.append('<table class="table">')
                html_lines.append('<thead>')
            
            cells = [c.strip() for c in line.split('|')[1:-1]]
            
            # 检查是否是分隔行
            if all(re.match(r'^[-:]+$', c) for c in cells):
                html_lines.append('</thead>')
                html_lines.append('<tbody>')
                i += 1
                continue
            
            row_tag = 'th' if '</thead>' not in '\n'.join(html_lines[-5:]) else 'td'
            html_lines.append('<tr>')
            for cell in cells:
                cell_html = inline_format(cell)
                html_lines.append(f'<{row_tag}>{cell_html}</{row_tag}>')
            html_lines.append('</tr>')
            i += 1
            continue
        elif in_table:
            in_table = False
            html_lines.append('</tbody>')
            html_lines.append('</table>')
        
        # 空行
        if not line.strip():
            if in_list:
                in_list = False
                html_lines.append(f'</{list_type}>')
            html_lines.append('')
            i += 1
            continue
        
        # 标题
        if line.startswith('#'):
            if in_list:
                in_list = False
                html_lines.append(f'</{list_type}>')
            level = len(re.match(r'^#+', line).group())
            text = line[level:].strip()
            text_html = inline_format(text)
            anchor = re.sub(r'[^\w\s-]', '', text.lower()).replace(' ', '-')
            html_lines.append(f'<h{level} id="{anchor}">{text_html}</h{level}>')
            i += 1
            continue
        
        # 无序列表
        if re.match(r'^[-*]\s', line.strip()):
            if not in_list or list_type != 'ul':
                if in_list:
                    html_lines.append(f'</{list_type}>')
                in_list = True
                list_type = 'ul'
                html_lines.append('<ul>')
            text = re.sub(r'^[-*]\s', '', line.strip())
            html_lines.append(f'<li>{inline_format(text)}</li>')
            i += 1
            continue
        
        # 有序列表
        if re.match(r'^\d+\.\s', line.strip()):
            if not in_list or list_type != 'ol':
                if in_list:
                    html_lines.append(f'</{list_type}>')
                in_list = True
                list_type = 'ol'
                html_lines.append('<ol>')
            text = re.sub(r'^\d+\.\s', '', line.strip())
            html_lines.append(f'<li>{inline_format(text)}</li>')
            i += 1
            continue
        
        # 分隔线
        if re.match(r'^[-*_]{3,}$', line.strip()):
            html_lines.append('<hr>')
            i += 1
            continue
        
        # 段落
        if in_list:
            in_list = False
            html_lines.append(f'</{list_type}>')
        html_lines.append(f'<p>{inline_format(line)}</p>')
        i += 1
    
    if in_list:
        html_lines.append(f'</{list_type}>')
    if in_table:
        html_lines.append('</tbody>')
        html_lines.append('</table>')
    
    return '\n'.join(html_lines)

def inline_format(text):
    """处理行内格式"""
    # 转义 HTML
    # text = escape_html(text)  # 暂时不转义，因为可能包含链接等
    
    # 粗体
    text = re.sub(r'\*\*(.+?)\*\*', r'<strong>\1</strong>', text)
    text = re.sub(r'__(.+?)__', r'<strong>\1</strong>', text)
    
    # 斜体
    text = re.sub(r'\*(.+?)\*', r'<em>\1</em>', text)
    text = re.sub(r'_(.+?)_', r'<em>\1</em>', text)
    
    # 行内代码
    text = re.sub(r'`([^`]+)`', r'<code>\1</code>', text)
    
    # 链接
    text = re.sub(r'\[([^\]]+)\]\(([^)]+)\)', r'<a href="\2">\1</a>', text)
    
    # 图片（徽章等）
    text = re.sub(r'!\[([^\]]*)\]\(([^)]+)\)', r'<img src="\2" alt="\1">', text)
    
    return text

def generate_html_page(title, content, css):
    """生成完整的 HTML 页面"""
    return f'''<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title}</title>
    <style>
{css}
    </style>
</head>
<body>
    <div class="container">
        <nav class="sidebar">
            <div class="sidebar-header">
                <h2>📖 导航</h2>
            </div>
            <div class="sidebar-content" id="toc">
            </div>
        </nav>
        <main class="content">
{content}
        </main>
    </div>
    <script>
        // 生成目录
        document.addEventListener('DOMContentLoaded', function() {{
            const toc = document.getElementById('toc');
            const headings = document.querySelectorAll('h1, h2, h3');
            let tocHtml = '<ul>';
            headings.forEach(function(heading) {{
                const level = parseInt(heading.tagName.charAt(1));
                const text = heading.textContent;
                const id = heading.id;
                const indent = (level - 1) * 15;
                tocHtml += `<li style="margin-left: ${{indent}}px"><a href="#${{id}}">${{text}}</a></li>`;
            }});
            tocHtml += '</ul>';
            toc.innerHTML = tocHtml;
        }});
    </script>
</body>
</html>'''

CSS = '''
:root {
    --bg-color: #1a1a2e;
    --sidebar-bg: #16213e;
    --content-bg: #0f0f23;
    --text-color: #e4e4e4;
    --heading-color: #00d9ff;
    --link-color: #64b5f6;
    --code-bg: #2d2d4a;
    --border-color: #3d3d5c;
    --table-header-bg: #2a2a4a;
    --success-color: #4caf50;
}

* {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
}

body {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
    background-color: var(--bg-color);
    color: var(--text-color);
    line-height: 1.6;
}

.container {
    display: flex;
    min-height: 100vh;
}

.sidebar {
    width: 280px;
    background-color: var(--sidebar-bg);
    border-right: 1px solid var(--border-color);
    position: fixed;
    height: 100vh;
    overflow-y: auto;
}

.sidebar-header {
    padding: 20px;
    border-bottom: 1px solid var(--border-color);
    background: linear-gradient(135deg, #1e3a5f 0%, #16213e 100%);
}

.sidebar-header h2 {
    color: var(--heading-color);
    font-size: 1.2rem;
}

.sidebar-content {
    padding: 15px;
}

.sidebar-content ul {
    list-style: none;
}

.sidebar-content li {
    margin: 5px 0;
}

.sidebar-content a {
    color: var(--text-color);
    text-decoration: none;
    font-size: 0.9rem;
    display: block;
    padding: 5px 10px;
    border-radius: 5px;
    transition: all 0.2s;
}

.sidebar-content a:hover {
    background-color: var(--code-bg);
    color: var(--heading-color);
}

.content {
    margin-left: 280px;
    padding: 40px 60px;
    max-width: 1000px;
    background-color: var(--content-bg);
    min-height: 100vh;
}

h1, h2, h3, h4, h5, h6 {
    color: var(--heading-color);
    margin: 1.5em 0 0.5em;
    font-weight: 600;
}

h1 {
    font-size: 2.5rem;
    border-bottom: 2px solid var(--heading-color);
    padding-bottom: 10px;
}

h2 {
    font-size: 1.8rem;
    border-bottom: 1px solid var(--border-color);
    padding-bottom: 8px;
}

h3 {
    font-size: 1.4rem;
}

h4 {
    font-size: 1.2rem;
}

p {
    margin: 1em 0;
}

a {
    color: var(--link-color);
    text-decoration: none;
}

a:hover {
    text-decoration: underline;
}

code {
    background-color: var(--code-bg);
    padding: 2px 6px;
    border-radius: 4px;
    font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
    font-size: 0.9em;
}

pre {
    background-color: var(--code-bg);
    padding: 15px 20px;
    border-radius: 8px;
    overflow-x: auto;
    margin: 1em 0;
    border: 1px solid var(--border-color);
}

pre code {
    background: none;
    padding: 0;
    font-size: 0.85rem;
    line-height: 1.5;
}

.table {
    width: 100%;
    border-collapse: collapse;
    margin: 1em 0;
}

.table th, .table td {
    border: 1px solid var(--border-color);
    padding: 10px 15px;
    text-align: left;
}

.table th {
    background-color: var(--table-header-bg);
    color: var(--heading-color);
    font-weight: 600;
}

.table tr:nth-child(even) {
    background-color: rgba(45, 45, 74, 0.3);
}

ul, ol {
    margin: 1em 0;
    padding-left: 2em;
}

li {
    margin: 0.5em 0;
}

hr {
    border: none;
    border-top: 1px solid var(--border-color);
    margin: 2em 0;
}

img {
    max-width: 100%;
    height: auto;
}

blockquote {
    border-left: 4px solid var(--heading-color);
    padding-left: 20px;
    margin: 1em 0;
    color: #aaa;
}

/* 响应式设计 */
@media (max-width: 768px) {
    .sidebar {
        display: none;
    }
    .content {
        margin-left: 0;
        padding: 20px;
    }
}

/* 代码高亮基础样式 */
.language-gql, .language-sql, .language-bash, .language-rust, .language-json {
    color: #a8d4ff;
}

/* 打印样式 */
@media print {
    .sidebar {
        display: none;
    }
    .content {
        margin-left: 0;
        background: white;
        color: black;
    }
    h1, h2, h3, h4 {
        color: #333;
    }
    pre, code {
        background-color: #f5f5f5;
        border: 1px solid #ddd;
    }
}
'''

def main():
    script_dir = Path(__file__).parent
    project_root = script_dir.parent
    docs_output = project_root / 'docs' / 'html'
    
    # 创建输出目录
    docs_output.mkdir(parents=True, exist_ok=True)
    
    # 转换 README.md
    readme_path = project_root / 'README.md'
    if readme_path.exists():
        print(f"正在转换: {readme_path}")
        with open(readme_path, 'r', encoding='utf-8') as f:
            md_content = f.read()
        html_content = markdown_to_html(md_content)
        html_page = generate_html_page('ChainGraph - README', html_content, CSS)
        output_path = docs_output / 'README.html'
        with open(output_path, 'w', encoding='utf-8') as f:
            f.write(html_page)
        print(f"  ✓ 已生成: {output_path}")
    
    # 转换 manual.md
    manual_path = project_root / 'docs' / 'manual.md'
    if manual_path.exists():
        print(f"正在转换: {manual_path}")
        with open(manual_path, 'r', encoding='utf-8') as f:
            md_content = f.read()
        html_content = markdown_to_html(md_content)
        html_page = generate_html_page('ChainGraph - 产品手册', html_content, CSS)
        output_path = docs_output / 'manual.html'
        with open(output_path, 'w', encoding='utf-8') as f:
            f.write(html_page)
        print(f"  ✓ 已生成: {output_path}")
    
    # 创建首页索引
    index_content = '''
# ChainGraph 文档中心

欢迎使用 ChainGraph 图数据库！

## 📚 文档列表

| 文档 | 描述 |
|------|------|
| [README](README.html) | 项目概述和快速入门 |
| [产品手册](manual.html) | 完整的产品使用手册 |

## 🚀 快速链接

- **GQL 查询语言** - 基于 ISO/IEC 39075 标准
- **图算法** - 最短路径、最大流、链路追踪
- **REST API** - 完整的 HTTP 接口

---

*ChainGraph - 专为 Web3 设计的高性能图数据库*
'''
    html_content = markdown_to_html(index_content)
    html_page = generate_html_page('ChainGraph 文档中心', html_content, CSS)
    output_path = docs_output / 'index.html'
    with open(output_path, 'w', encoding='utf-8') as f:
        f.write(html_page)
    print(f"  ✓ 已生成: {output_path}")
    
    print("\n文档生成完成！")
    print(f"文档目录: {docs_output}")

if __name__ == '__main__':
    main()
